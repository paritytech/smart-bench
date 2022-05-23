mod transaction;
mod xts;

use crate::moonbeam::xts::MoonbeamApi;
use crate::Cli;
use color_eyre::{eyre, Section as _};

pub async fn exec(cli: &Cli) -> color_eyre::Result<()> {
    // incrementer
    let name = "incrementer";

    let root = std::env::var("CARGO_MANIFEST_DIR")?;
    let bin_path = format!("{root}/contracts/solidity/{name}.bin");
    let metadata_path = format!("{root}/contracts/solidity/{name}_meta.json");
    let code = std::fs::read(bin_path)?;
    let metadata_reader = std::fs::File::open(metadata_path)?;
    let json: serde_json::Map<String, serde_json::Value> =
        serde_json::from_reader(metadata_reader)?;
    let abi = json["output"]["abi"].clone();
    let contract: ethabi::Contract = serde_json::from_value(abi)?;

    let constructor = contract
        .constructor()
        .ok_or_else(|| eyre::eyre!("No constructor for contract found"))?;
    let data = constructor.encode_input(code.into(), &[ethabi::Token::Uint(0u32.into())])?;

    let ws = web3::transports::ws::WebSocket::new(&cli.url).await?;
    let api = MoonbeamApi::new(ws);
    api.deploy(data, &xts::alice()).await?;

    // println!("Created new contract {:?}", contract_account);

    Ok(())
}
