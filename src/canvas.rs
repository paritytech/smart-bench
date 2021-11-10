use codec::Encode;
use color_eyre::eyre;
use sp_core::sr25519;
use subxt::PairSigner;

use super::*;

type Balance = u128;
type Gas = u64;
type ContractAccount = <api::DefaultConfig as subxt::Config>::AccountId;
type Hash = <api::DefaultConfig as subxt::Config>::Hash;
type Signer = PairSigner<api::DefaultConfig, sr25519::Pair>;

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
    constructor: C,
    salt: Vec<u8>,
    signer: &Signer,
) -> color_eyre::Result<ContractAccount> {
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
    contract: ContractAccount,
    value: Balance,
    gas_limit: Gas,
    message: M,
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
