mod account;
mod transaction;
mod xts;

use crate::Cli;
use color_eyre::{Section as _, eyre};
use web3::{
    Web3,
    signing::SecretKeyRef,
    transports::ws,
    types::{Address, Bytes, TransactionParameters, U64, U256, H256},
};
use secp256k1::SecretKey;
use web3::signing::Key;
use transaction::{
    Transaction,
    EIP1559_TX_ID,
};
use std::str::FromStr;

pub async fn exec(cli: &Cli) -> color_eyre::Result<()> {
    let ws = ws::WebSocket::new(&cli.url).await?;
    let web3 = Web3::new(ws);

    // incrementer
    let name = "incrementer";

    let root = std::env::var("CARGO_MANIFEST_DIR")?;
    let bin_path = format!("{root}/contracts/solidity/{name}.bin");
    let metadata_path = format!("{root}/contracts/solidity/{name}_meta.json");
    let code = std::fs::read(bin_path)?;
    let metadata_reader = std::fs::File::open(metadata_path)?;
    let json: serde_json::Map<String, serde_json::Value> = serde_json::from_reader(metadata_reader)?;
    let abi = json["output"]["abi"].clone();
    let contract: ethabi::Contract = serde_json::from_value(abi)?;

    let secret_key = SecretKey::from_str("5fb92d6e98884f76de468fa3f6278f8807c48bebc13595d45af5bdc4da702133")?;
    deploy(&web3, &secret_key, code, contract).await?;

    // println!("Created new contract {:?}", contract_account);

    Ok(())
}

pub async fn deploy(web3: &Web3<ws::WebSocket>, origin: impl Key, code: Vec<u8>, contract: ethabi::Contract) -> color_eyre::Result<H256> {
    let constructor = contract
        .constructor()
        .ok_or_else(|| eyre::eyre!("No constructor for contract found"))?;
    let data = constructor.encode_input(code.into(), &[ethabi::Token::Uint(0u32.into())])?;

    let nonce = web3.eth().transaction_count(origin.address(), None).await?;
    let gas_price = web3.eth().gas_price().await?;
    let chain_id = web3.eth().chain_id().await?;

    // let max_priority_fee_per_gas = match tx.transaction_type {
    //     Some(tx_type) if tx_type == U64::from(EIP1559_TX_ID) => {
    //         tx.max_priority_fee_per_gas.unwrap_or(gas_price)
    //     }
    //     _ => gas_price,
    // };

    let tx = Transaction {
        nonce,
        to: None,
        gas: 1_000_000u32.into(),
        gas_price,
        value: 0u32.into(),
        data: data.into(),
        transaction_type: None,
        access_list: Default::default(),
        max_priority_fee_per_gas: gas_price,
    };

    let signed_tx = tx.sign(origin, chain_id.as_u64());
    let hash = web3.eth().send_raw_transaction(signed_tx.raw_transaction).await?;
    Ok(hash)
}