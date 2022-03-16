use super::*;
use crate::blocks;
use codec::Encode;
use color_eyre::eyre;
use subxt::Signer as _;

pub const DEFAULT_STORAGE_DEPOSIT_LIMIT: Option<Balance> = None;

pub struct BenchRunner {
    url: String,
    api: ContractsApi,
    gas_limit: Gas,
    signer: Signer,
    calls: Vec<(String, Vec<Call>)>,
}

impl BenchRunner {
    pub async fn new(mut signer: Signer, gas_limit: Gas, url: &str) -> color_eyre::Result<Self> {
        let client = subxt::ClientBuilder::new().set_url(url).build().await?;

        let nonce = client
            .rpc()
            .system_account_next_index(signer.account_id())
            .await?;
        signer.set_nonce(nonce);

        let api = ContractsApi::new(client);

        Ok(Self {
            url: url.to_string(),
            api,
            signer,
            gas_limit,
            calls: Vec::new(),
        })
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
        println!("Preparing {name}");

        let root = std::env::var("CARGO_MANIFEST_DIR")?;
        let contract_path = format!("contracts/{name}.contract");
        let metadata_path: std::path::PathBuf = [&root, &contract_path].iter().collect();
        let reader = std::fs::File::open(metadata_path)?;
        let contract: contract_metadata::ContractMetadata = serde_json::from_reader(reader)?;
        let code = contract
            .source
            .wasm
            .ok_or_else(|| eyre::eyre!("contract bundle missing source Wasm"))?;

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

        let mut accounts = Vec::new();
        for i in 0..count {
            let code = append_unique_name_section(&code, i)?;
            let salt = Vec::new();

            let contract = self
                .api
                .instantiate_with_code(
                    value,
                    self.gas_limit,
                    DEFAULT_STORAGE_DEPOSIT_LIMIT,
                    code,
                    data.clone(),
                    salt,
                    &mut self.signer,
                )
                .await?;
            accounts.push(contract);
            self.signer.increment_nonce();
        }

        Ok(accounts)
    }

    /// Call each contract instance `call_count` times. Wait for all txs to be included in a block
    /// before returning.
    pub async fn run(
        &mut self,
        call_count: u32,
    ) -> color_eyre::Result<impl futures::Stream<Item = blocks::BlockExtrinsics>> {
        let block_subscription = blocks::BlocksSubscription::new(&self.url).await?;

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
                        let tx_hash = self
                            .api
                            .call(
                                contract_call.contract_account.clone(),
                                0,
                                self.gas_limit,
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

        Ok(block_subscription.wait_for_txs(&tx_hashes))
    }
}

/// Add a custom section to make the Wasm code unique to upload many copies of the same contract.
fn append_unique_name_section(code: &[u8], instance_id: u32) -> color_eyre::Result<Vec<u8>> {
    let mut module: parity_wasm::elements::Module = parity_wasm::deserialize_buffer(code)?;
    module.set_custom_section("smart-bench-unique", instance_id.to_le_bytes().to_vec());
    let code = module.to_bytes()?;
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
