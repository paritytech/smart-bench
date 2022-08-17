use super::*;
use codec::Encode;
use color_eyre::eyre;
use futures::{future, StreamExt, TryStream, TryStreamExt};
use sp_runtime::traits::{BlakeTwo256, Hash as _};
use std::time::{SystemTime, UNIX_EPOCH};

pub const DEFAULT_STORAGE_DEPOSIT_LIMIT: Option<Balance> = None;

pub struct BenchRunner {
    url: String,
    api: ContractsApi,
    signer: Signer,
    calls: Vec<(String, Vec<Call>)>,
}

impl BenchRunner {
    pub async fn new(signer: Signer, url: &str) -> color_eyre::Result<Self> {
        let client = subxt::OnlineClient::from_url(url).await?;

        let api = ContractsApi::new(client, url).await?;

        let mut runner = Self {
            url: url.to_string(),
            api,
            signer,
            calls: Vec::new(),
        };
        runner.set_nonce().await?;
        Ok(runner)
    }

    async fn set_nonce(&mut self) -> color_eyre::Result<()> {
        let nonce = self
            .api
            .client
            .rpc()
            .system_account_next_index(self.signer.account_id())
            .await?;
        self.signer.set_nonce(nonce);
        Ok(())
    }

    /// Upload and instantiate instances of contract, and build calls for benchmarking
    pub async fn prepare_contract<C, F>(
        &mut self,
        name: &str,
        constructor: C,
        instance_count: u32,
        mut create_message: F,
    ) -> color_eyre::Result<()>
    where
        C: InkConstructor,
        F: FnMut() -> EncodedMessage,
    {
        print!("Preparing {name}...");

        let root = std::env::var("CARGO_MANIFEST_DIR")?;
        let contract_path = format!("contracts/{name}.contract");
        let metadata_path: std::path::PathBuf = [&root, &contract_path].iter().collect();
        let reader = std::fs::File::open(metadata_path)?;
        let contract: contract_metadata::ContractMetadata = serde_json::from_reader(reader)?;
        let code = contract
            .source
            .wasm
            .ok_or_else(|| eyre::eyre!("contract bundle missing source Wasm"))?;

        println!("{}KiB", code.0.len() / 1024);

        self.set_nonce().await?;

        let contract_accounts = self
            .exec_instantiate(0, code.0, &constructor, instance_count)
            .await?;

        println!("Instantiated {} {name} contracts", contract_accounts.len());

        let calls = contract_accounts
            .iter()
            .map(|contract| {
                let message = create_message();
                Call {
                    contract_account: contract.clone(),
                    call_data: message,
                }
            })
            .collect::<Vec<_>>();

        self.calls.push((name.to_string(), calls));

        Ok(())
    }

    async fn exec_instantiate<C: InkConstructor>(
        &mut self,
        value: Balance,
        code: Vec<u8>,
        constructor: &C,
        count: u32,
    ) -> color_eyre::Result<Vec<AccountId>> {
        let mut data = C::SELECTOR.to_vec();
        <C as Encode>::encode_to(constructor, &mut data);

        // a value to append to a contract's custom section to make the code unique
        let unique_code_salt = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis();

        // dry run the instantiate to calculate the gas limit
        let gas_limit = {
            let code = append_unique_name_section(&code, unique_code_salt)?;
            let dry_run = self
                .api
                .instantiate_with_code_dry_run(
                    value,
                    DEFAULT_STORAGE_DEPOSIT_LIMIT,
                    code,
                    data.clone(),
                    Vec::new(),
                    &self.signer,
                )
                .await?;
            dry_run.gas_required
        };

        let mut event_sub =
            self.api.client.events().subscribe().await?;

        let mut accounts = Vec::new();
        for i in unique_code_salt..unique_code_salt + count as u128 {
            let code = append_unique_name_section(&code, i)?;
            let salt = Vec::new();

            self.api
                .instantiate_with_code(
                    value,
                    gas_limit,
                    DEFAULT_STORAGE_DEPOSIT_LIMIT,
                    code,
                    data.clone(),
                    salt,
                    &mut self.signer,
                )
                .await?;
            self.signer.increment_nonce();
        }

        while let Some(Ok(events)) = event_sub.next().await {
            for event in events.iter() {
                let event = event?;
                if let Some(instantiated) = event.as_event::<xts::api::contracts::events::Instantiated>()? {
                    accounts.push(instantiated.contract);
                    if accounts.len() == count as usize {
                        break;
                    }
                } else if event.as_event::<xts::api::system::events::ExtrinsicFailed>()?.is_some() {
                    let metadata = self.api.client.metadata();
                    let dispatch_error =
                        subxt::error::DispatchError::decode_from(event.field_bytes(), &metadata);
                    return Err(eyre::eyre!("Instantiate Extrinsic Failed: {:?}", dispatch_error));
                }
            }
        }

        Ok(accounts)
    }

    /// Call each contract instance `call_count` times. Wait for all txs to be included in a block
    /// before returning.
    pub async fn run(
        &mut self,
        call_count: u32,
    ) -> color_eyre::Result<impl TryStream<Ok = BlockInfo, Error = color_eyre::Report> + '_> {
        let block_stats = blockstats::subscribe_stats(&self.url).await?;

        let mut tx_hashes = Vec::new();
        let max_instance_count = self
            .calls
            .iter()
            .map(|(_, calls)| calls.len())
            .max()
            .ok_or_else(|| eyre::eyre!("No prepared contracts for benchmarking."))?;

        for _ in 0..call_count {
            for i in 0..max_instance_count {
                for (_name, contract_calls) in &self.calls {
                    if let Some(contract_call) = contract_calls.get(i as usize) {
                        // dry run the call to calculate the gas limit
                        let gas_limit = {
                            let dry_run = self
                                .api
                                .call_dry_run(
                                    contract_call.contract_account.clone(),
                                    0,
                                    DEFAULT_STORAGE_DEPOSIT_LIMIT,
                                    contract_call.call_data.0.clone(),
                                    &self.signer,
                                )
                                .await?;
                            dry_run.gas_required
                        };

                        let tx_hash = self
                            .api
                            .call(
                                contract_call.contract_account.clone(),
                                0,
                                gas_limit,
                                DEFAULT_STORAGE_DEPOSIT_LIMIT,
                                contract_call.call_data.0.clone(),
                                &self.signer,
                            )
                            .await?;
                        self.signer.increment_nonce();
                        tx_hashes.push(tx_hash)
                    }
                }
            }
        }

        println!("Submitted {} total contract calls", tx_hashes.len());

        let mut remaining_hashes: std::collections::HashSet<Hash> =
            tx_hashes.iter().cloned().collect();

        let wait_for_txs = block_stats
            .map_err(|e| eyre::eyre!("Block stats subscription error: {e:?}"))
            .and_then(|stats| {
                tracing::debug!("{stats:?}");
                let client = self.api.client.clone();
                async move {
                    let block = client.rpc().block(Some(stats.hash)).await?;
                    let extrinsics = block
                        .unwrap_or_else(|| panic!("block {} not found", stats.hash))
                        .block
                        .extrinsics
                        .iter()
                        .map(BlakeTwo256::hash_of)
                        .collect();
                    Ok(BlockInfo { extrinsics, stats })
                }
            })
            .try_take_while(move |block_info| {
                let some_remaining_txs = !remaining_hashes.is_empty();
                for xt in &block_info.extrinsics {
                    remaining_hashes.remove(xt);
                }
                future::ready(Ok(some_remaining_txs))
            });

        Ok(wait_for_txs)
    }
}

/// Add a custom section to make the Wasm code unique to upload many copies of the same contract.
fn append_unique_name_section(code: &[u8], instance_id: u128) -> color_eyre::Result<Vec<u8>> {
    let mut module: parity_wasm::elements::Module = parity_wasm::deserialize_buffer(code)?;
    module.set_custom_section("smart-bench-unique", instance_id.to_le_bytes().to_vec());
    let code = module.into_bytes()?;
    Ok(code)
}

#[derive(Clone)]
pub struct EncodedMessage(Vec<u8>);

impl EncodedMessage {
    fn new<M: InkMessage>(call: &M) -> Self {
        let mut call_data = M::SELECTOR.to_vec();
        <M as Encode>::encode_to(call, &mut call_data);
        Self(call_data)
    }
}

impl<M> From<M> for EncodedMessage
where
    M: InkMessage,
{
    fn from(msg: M) -> Self {
        EncodedMessage::new(&msg)
    }
}

#[derive(Clone)]
pub struct Call {
    contract_account: AccountId,
    call_data: EncodedMessage,
}

pub struct BlockInfo {
    pub stats: blockstats::BlockStats,
    pub extrinsics: Vec<Hash>,
}
