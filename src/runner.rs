use super::*;
use crate::canvas::ExtrinsicsResult;
use codec::Encode;
use color_eyre::eyre;
use subxt::Signer as _;

pub struct BenchRunner {
    api: canvas::ContractsApi,
    gas_limit: canvas::Gas,
    signer: canvas::Signer,
    calls: Vec<(String, Vec<Call>)>,
}

impl BenchRunner {
    pub async fn new(
        mut signer: canvas::Signer,
        gas_limit: canvas::Gas,
        url: &str,
    ) -> color_eyre::Result<Self> {
        let client = subxt::ClientBuilder::new().set_url(url).build().await?;

        let nonce = client
            .fetch_nonce::<canvas::api::DefaultAccountData>(signer.account_id())
            .await?;
        signer.set_nonce(nonce);

        let api = canvas::ContractsApi::new(client);

        Ok(Self {
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
        value: canvas::Balance,
        code: Vec<u8>,
        constructor: &C,
        count: u32,
    ) -> color_eyre::Result<Vec<canvas::AccountId>> {
        let mut data = C::SELECTOR.to_vec();
        <C as Encode>::encode_to(constructor, &mut data);

        let mut accounts = Vec::new();
        for i in 0..count {
            let salt = i.to_le_bytes().to_vec();

            let contract = self
                .api
                .instantiate_with_code(
                    value,
                    self.gas_limit,
                    DEFAULT_STORAGE_DEPOSIT_LIMIT,
                    code.clone(),
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
    ) -> color_eyre::Result<ExtrinsicsResult> {
        let block_subscription = canvas::BlocksSubscription::new().await?;

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

        block_subscription.wait_for_txs(&tx_hashes).await
    }
}
