use codec::Encode;
use color_eyre::eyre;
use sp_core::sr25519;
use subxt::PairSigner;

use super::*;

pub type Balance = u128;
pub type Gas = u64;
pub type AccountId = <api::DefaultConfig as subxt::Config>::AccountId;
pub type Hash = <api::DefaultConfig as subxt::Config>::Hash;
pub type Header = <api::DefaultConfig as subxt::Config>::Header;
pub type Signer = PairSigner<api::DefaultConfig, sr25519::Pair>;

#[subxt::subxt(runtime_metadata_path = "metadata/canvas.scale")]
pub mod api {}

async fn api() -> color_eyre::Result<api::RuntimeApi<api::DefaultConfig>> {
    Ok(subxt::ClientBuilder::new()
        // .set_url()
        .build()
        .await?
        .to_runtime_api::<api::RuntimeApi<api::DefaultConfig>>())
}

/// Submit extrinsic to instantiate a contract with the given code.
pub async fn instantiate_with_code<C: InkConstructor>(
    endowment: Balance,
    gas_limit: Gas,
    code: Vec<u8>,
    constructor: &C,
    salt: Vec<u8>,
    signer: &Signer,
) -> color_eyre::Result<AccountId> {
    let api = api().await?;

    let mut data = C::SELECTOR.to_vec();
    <C as Encode>::encode_to(&constructor, &mut data);

    let result = api
        .tx()
        .contracts()
        .instantiate_with_code(endowment, gas_limit, code, data, salt)
        .sign_and_submit_then_watch(signer)
        .await?;

    let instantiated = result
        .find_event::<api::contracts::events::Instantiated>()?
        .ok_or(eyre::eyre!("Failed to find Instantiated event"))?;

    Ok(instantiated.1)
}

/// Submit extrinsic to call a contract.
pub async fn call<M: InkMessage>(
    contract: AccountId,
    value: Balance,
    gas_limit: Gas,
    message: &M,
    signer: &Signer,
) -> color_eyre::Result<Hash> {
    let api = api().await?;

    let mut data = M::SELECTOR.to_vec();
    <M as Encode>::encode_to(&message, &mut data);

    let tx_hash = api
        .tx()
        .contracts()
        .call(contract.into(), value, gas_limit, data)
        .sign_and_submit(signer)
        .await?;

    Ok(tx_hash)
}

pub struct BlocksSubscription {
    task: async_std::task::JoinHandle<()>,
}

impl BlocksSubscription {
    pub async fn new() -> color_eyre::Result<Self> {
        let client: subxt::Client<api::DefaultConfig> = subxt::ClientBuilder::new()
            .build()
            .await?;
        let mut blocks_sub: jsonrpsee_types::Subscription<Header> = client.rpc().subscribe_blocks().await?;

        let task = async_std::task::spawn(async move {
            while let Ok(Some(block_header)) = blocks_sub.next().await {
                if let Ok(Some(block)) = client.rpc().block(Some(block_header.hash())).await {
                    println!("Block {}, Extrinsics {}", block_header.number, block.block.extrinsics.len());
                }
            }
        });

        Ok(Self { task })
    }
}
