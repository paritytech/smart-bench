mod account;
mod xts;

use crate::Cli;
use color_eyre::eyre;
use ethabi::Contract;
use sp_core::{H256, U256};

pub async fn exec(cli: &Cli) -> color_eyre::Result<()> {
    let client = subxt::ClientBuilder::new()
        .set_url(&cli.url)
        .build()
        .await?;
    let api = xts::MoonbeamApi::new(client);

    // incrementer
    let name = "incrementer";

    let root = std::env::var("CARGO_MANIFEST_DIR")?;
    let bin_path = format!("{root}/contracts/solidity/{name}.bin");
    let metadata_path = format!("{root}/contracts/solidity/{name}_meta.json");
    let code = std::fs::read(bin_path)?;
    let metadata_reader = std::fs::File::open(metadata_path)?;

    let contract = Contract::load(metadata_reader)?;
    let constructor = contract
        .constructor()
        .ok_or_else(|| eyre::eyre!("No constructor for contract found"))?;
    let data = constructor.encode_input(code.into(), &[ethabi::Token::Uint(0u32.into())])?;
    let salt = H256::zero();
    let value = U256::zero();
    let gas_limit = 21_000_000;
    let nonce = None;
    let signer = xts::alice();

    let contract_account = api
        .create2(data, salt, value, gas_limit, nonce, &signer)
        .await?;

    println!("Created new contract {:?}", contract_account);

    Ok(())
}
