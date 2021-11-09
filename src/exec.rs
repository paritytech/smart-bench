use color_eyre::eyre;
use sp_core::sr25519;
use subxt::PairSigner;

use super::*;

smart_bench_macro::contract!("/home/andrew/code/paritytech/ink/examples/erc20");

pub async fn instantiate_with_code(
    endowment: Balance,
    gas_limit: Gas,
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
        .find_event::<canvas::contracts::events::Instantiated>()?
        .ok_or(eyre::eyre!("Failed to find Instantiated event"))?;

    Ok(instantiated.1)
}

pub async fn call(
    contract: ContractAccount,
    value: Balance,
    gas_limit: Gas,
    data: Vec<u8>,
    signer: &Signer,
) -> color_eyre::Result<Hash> {
    let api = api().await?;

    let tx_hash = api
        .tx()
        .contracts()
        .call(contract.into(), value, gas_limit, data)
        .sign_and_submit(signer)
        .await?;

    Ok(tx_hash)
}
