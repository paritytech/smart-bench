use color_eyre::eyre;
use sp_core::sr25519;
use subxt::PairSigner;

#[subxt::subxt(runtime_metadata_path = "metadata/contracts_runtime.scale")]
pub mod api {}

type ContractAccount = <api::DefaultConfig as subxt::Config>::AccountId;
type Signer = PairSigner<api::DefaultConfig, sr25519::Pair>;

#[async_std::main]
async fn main() -> color_eyre::Result<()> {
    println!("Hello, world!");
    Ok(())
}

async fn api() -> color_eyre::Result<api::RuntimeApi<api::DefaultConfig>> {
    Ok(subxt::ClientBuilder::new()
        // .set_url()
        .build()
        .await?
        .to_runtime_api::<api::RuntimeApi<api::DefaultConfig>>())
}

async fn instantiate_with_code(
    endowment: u128,
    gas_limit: u64,
    code: Vec<u8>,
    data: Vec<u8>,
    salt: Vec<u8>,
    signer: &Signer,
) -> color_eyre::Result<ContractAccount> {
    let api = api().await?;

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

fn call() -> color_eyre::Result<()> {
    todo!()
}
