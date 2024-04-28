use self::xts::api;

use super::*;
use crate::BlockInfo;
use codec::Encode;
use color_eyre::eyre;
use futures::{TryStream, StreamExt};
use sp_runtime::traits::{BlakeTwo256, Hash as _};
use std::time::{SystemTime, UNIX_EPOCH};
use subxt::{backend::rpc::RpcClient, OnlineClient, PolkadotConfig as DefaultConfig};

use xts::api::{
    contracts::calls::types::Call, contracts::events::Instantiated, system::events::ExtrinsicFailed,
};

pub const DEFAULT_STORAGE_DEPOSIT_LIMIT: Option<Balance> = None;

pub struct BenchRunner {
    url: String,
    api: ContractsApi,
    signer: Signer,
    calls: Vec<(String, Vec<RunnerCall>)>,
    pub call_signers: Option<Vec<Signer>>,
}

impl BenchRunner {
    pub async fn new(signer: Signer, url: &str, call_signers: Option<Vec<Signer>>) -> color_eyre::Result<Self> {
        let client = RpcClient::from_url(url).await?;

        let api = ContractsApi::new(client).await?;

        let runner = Self {
            url: url.to_string(),
            api,
            signer,
            calls: Vec::new(),
            call_signers
        };
        Ok(runner)
    }

    /// Upload and instantiate instances of contract, and build calls for benchmarking
    pub async fn prepare_contract<C, F>(
        &mut self,
        path: &str,
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
        let contract_path = format!("{path}/{name}.contract");
        let metadata_path: std::path::PathBuf = [&root, &contract_path].iter().collect();
        let reader = std::fs::File::open(metadata_path)?;
        let contract: contract_metadata::ContractMetadata = serde_json::from_reader(reader)?;
        let code = contract
            .source
            .wasm
            .ok_or_else(|| eyre::eyre!("contract bundle missing source Wasm"))?;

        println!("{}KiB", code.0.len() / 1024);

        let contract_accounts = self
            .exec_instantiate(0, code.0, &constructor, instance_count)
            .await?;

        println!("Instantiated {} {name} contracts", contract_accounts.len());

        let calls = contract_accounts
            .iter()
            .map(|contract| {
                let message = create_message();
                RunnerCall {
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
                .await;
            dry_run.gas_required
        };

        let mut block_sub = self.api.client.blocks().subscribe_best().await?;

        let mut accounts = Vec::new();
        for i in unique_code_salt..unique_code_salt + count as u128 {
            let code = append_unique_name_section(&code, i)?;
            let salt = Vec::new();

            self.api
                .instantiate_with_code(
                    value,
                    gas_limit.into(),
                    DEFAULT_STORAGE_DEPOSIT_LIMIT,
                    code,
                    data.clone(),
                    salt,
                    &mut self.signer,
                )
                .await?;
        }

        while let Some(Ok(block)) = block_sub.next().await {
            let events = block.events().await?;
            for event in events.iter() {
                let event = event?;
                if let Some(instantiated) = event.as_event::<Instantiated>()? {
                    accounts.push(instantiated.contract);
                    if accounts.len() == count as usize {
                        return Ok(accounts);
                    }
                } else if event.as_event::<ExtrinsicFailed>()?.is_some() {
                    let metadata = self.api.client.metadata();
                    let dispatch_error =
                        subxt::error::DispatchError::decode_from(event.field_bytes(), metadata);
                    return Err(eyre::eyre!(
                        "Instantiate Extrinsic Failed: {:?}",
                        dispatch_error
                    ));
                }
            }
        }
        Err(eyre::eyre!(
            "Expected {} Instantiated events, received {}",
            count,
            accounts.len()
        ))
    }

    async fn get_block_details(
        client: OnlineClient<DefaultConfig>,
        block_hash: sp_core::H256,
    ) -> color_eyre::Result<(u64, Vec<sp_core::H256>)> {
        let block = client.blocks().at(block_hash).await?;
        let mut tx_hashes = Vec::new();
        let extrinsics_details = block
            .extrinsics()
            .await?
            .iter()
            .collect::<Result<Vec<_>, _>>()?;

        for extrinsic_detail in extrinsics_details {
            if let Some(Call { .. }) = extrinsic_detail.as_extrinsic::<Call>()? {
                tx_hashes.push(BlakeTwo256::hash_of(&extrinsic_detail.bytes()));
            }
        }
        let storage_timestamp_storage_addr = api::storage().timestamp().now();
        let time_stamp = client
            .storage()
            .at(block_hash)
            .fetch(&storage_timestamp_storage_addr)
            .await?
            .unwrap();
        Ok((time_stamp, tx_hashes))
    }

    /// Call each contract instance `call_count` times. Wait for all txs to be included in a block
    /// before returning.
    pub async fn run(
        &mut self,
        call_count: u32,
    ) -> color_eyre::Result<impl TryStream<Ok = BlockInfo, Error = color_eyre::Report> + '_> {
        let block_stats = blockstats::subscribe_stats(&self.url).await?;

        let max_instance_count = self
            .calls
            .iter()
            .map(|(_, calls)| calls.len())
            .max()
            .ok_or_else(|| eyre::eyre!("No prepared contracts for benchmarking."))?;

        println!("Start benchmark calls");
        let mut counter = 0;
        let mut futures = vec![];
        for _ in 0..call_count {
            for i in 0..max_instance_count {
                for (_name, contract_calls) in &self.calls {

                    let signer = if let Some(signers) = &self.call_signers {
                        counter+=1;
                        &signers[counter-1]
                    } else {
                        &self.signer
                    };
                    
                    if let Some(contract_call) = contract_calls.get(i as usize) {
                        // dry run the call to calculate the gas limit
                        let mut gas_limit = {
                            let dry_run = self
                                .api
                                .call_dry_run(
                                    contract_call.contract_account.clone(),
                                    0,
                                    DEFAULT_STORAGE_DEPOSIT_LIMIT,
                                    contract_call.call_data.0.clone(),
                                    &signer,
                                )
                                .await?;
                            dry_run.gas_required
                        };

                        // extra 5% of gas limit
                        // due to "not enough gas" rpc errors
                        gas_limit = gas_limit.checked_mul(105).expect("Gas limit overflow") / 100;

                        let tx_hash = self
                            .api
                            .call(
                                contract_call.contract_account.clone(),
                                0,
                                gas_limit.into(),
                                DEFAULT_STORAGE_DEPOSIT_LIMIT,
                                contract_call.call_data.0.clone(),
                                &signer
                            );
                        futures.push(tx_hash);
                    }
                }
            }
        }

        const MAX_PARALLEL_RPC_CONN: usize = 100;
        let stream = futures::stream::iter(futures).buffer_unordered(MAX_PARALLEL_RPC_CONN);
        let tx_hashes = stream.collect::<Vec<_>>().await;
        println!("Submitted {} total contract calls", tx_hashes.len());

        let remaining_hashes: std::collections::HashSet<Hash> = tx_hashes.into_iter().collect::<Result<Vec<_>, _>>()?.into_iter().collect();
        println!("remaining_hashes {} ", remaining_hashes.len());

        let wait_for_txs = crate::collect_block_stats(block_stats, remaining_hashes, |hash| {
            let client = self.api.client.clone();
            Self::get_block_details(client, hash)
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
pub struct RunnerCall {
    contract_account: AccountId,
    call_data: EncodedMessage,
}
