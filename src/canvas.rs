use color_eyre::eyre;
use futures::{SinkExt, StreamExt};
use sp_core::sr25519;
use sp_runtime::traits::{BlakeTwo256, Hash as _};
use subxt::{DefaultConfig, DefaultExtra, PairSigner};

pub type Balance = u128;
pub type Gas = u64;
pub type AccountId = <DefaultConfig as subxt::Config>::AccountId;
pub type Hash = <DefaultConfig as subxt::Config>::Hash;
pub type Signer = PairSigner<DefaultConfig, DefaultExtra<DefaultConfig>, sr25519::Pair>;

#[subxt::subxt(runtime_metadata_path = "metadata/canvas.scale")]
pub mod api {}

pub struct ContractsApi {
    api: api::RuntimeApi<DefaultConfig, DefaultExtra<DefaultConfig>>,
}

impl ContractsApi {
    pub fn new(client: subxt::Client<DefaultConfig>) -> Self {
        let api =
            client.to_runtime_api::<api::RuntimeApi<DefaultConfig, DefaultExtra<DefaultConfig>>>();
        Self { api }
    }

    /// Submit extrinsic to instantiate a contract with the given code.
    pub async fn instantiate_with_code(
        &self,
        value: Balance,
        gas_limit: Gas,
        storage_deposit_limit: Option<Balance>,
        code: Vec<u8>,
        data: Vec<u8>,
        salt: Vec<u8>,
        signer: &Signer,
    ) -> color_eyre::Result<AccountId> {
        let result = self
            .api
            .tx()
            .contracts()
            .instantiate_with_code(value, gas_limit, storage_deposit_limit, code, data, salt)
            .sign_and_submit_then_watch(signer)
            .await?
            .wait_for_in_block()
            .await?
            .wait_for_success()
            .await?;

        let instantiated = result
            .find_first::<api::contracts::events::Instantiated>()?
            .ok_or_else(|| eyre::eyre!("Failed to find Instantiated event"))?;

        Ok(instantiated.contract)
    }

    /// Submit extrinsic to call a contract.
    pub async fn call(
        &self,
        contract: AccountId,
        value: Balance,
        gas_limit: Gas,
        storage_deposit_limit: Option<Balance>,
        data: Vec<u8>,
        signer: &Signer,
    ) -> color_eyre::Result<Hash> {
        let tx_hash = self
            .api
            .tx()
            .contracts()
            .call(
                contract.into(),
                value,
                gas_limit,
                storage_deposit_limit,
                data,
            )
            .sign_and_submit(signer)
            .await?;

        Ok(tx_hash)
    }
}

pub struct BlocksSubscription {
    receiver: futures::channel::mpsc::UnboundedReceiver<BlockExtrinsics>,
}

impl BlocksSubscription {
    pub fn wait_for_txs(self, tx_hashes: &[Hash]) -> impl futures::Stream<Item = BlockExtrinsics> {
        let mut remaining_hashes: std::collections::HashSet<Hash> =
            tx_hashes.iter().cloned().collect();

        self.receiver.take_while(move |block_xts| {
            let some_remaining_txs = !remaining_hashes.is_empty();
            for xt in &block_xts.extrinsics {
                remaining_hashes.remove(xt);
            }
            futures::future::ready(some_remaining_txs)
        })
    }
}

impl BlocksSubscription {
    pub async fn new(url: &str) -> color_eyre::Result<Self> {
        let client: subxt::Client<DefaultConfig> =
            subxt::ClientBuilder::new().set_url(url).build().await?;
        let mut blocks_sub = client.rpc().subscribe_blocks().await?;
        let (mut sender, receiver) = futures::channel::mpsc::unbounded();

        tokio::task::spawn(async move {
            while let Some(Ok(block_header)) = blocks_sub.next().await {
                if let Ok(Some(block)) = client.rpc().block(Some(block_header.hash())).await {
                    let extrinsics = block
                        .block
                        .extrinsics
                        .iter()
                        .map(BlakeTwo256::hash_of)
                        .collect();
                    let block_extrinsics = BlockExtrinsics {
                        block_number: block_header.number,
                        block_hash: block_header.hash(),
                        extrinsics,
                    };
                    sender.send(block_extrinsics).await.expect("Send failed");
                }
            }
        });

        Ok(Self { receiver })
    }
}

#[derive(Debug)]
pub struct BlockExtrinsics {
    pub block_number: u32,
    pub block_hash: Hash,
    pub extrinsics: Vec<Hash>,
}
