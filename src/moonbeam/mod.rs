mod runner;
mod transaction;
mod xts;

use crate::{
    moonbeam::{runner::MoonbeamRunner, xts::MoonbeamApi},
    Cli,
};
use web3::contract::tokens::Tokenize;

pub async fn exec(cli: &Cli) -> color_eyre::Result<()> {
    let params = (1u32,).into_tokens();
    let api = MoonbeamApi::new(&cli.url).await?;

    let mut runner = MoonbeamRunner::new(keyring::alith(), api);

    runner
        .prepare_contract("incrementer", cli.instance_count, &params)
        .await?;

    Ok(())
}

mod keyring {
    use secp256k1::SecretKey;
    use std::str::FromStr as _;

    pub fn alith() -> SecretKey {
        SecretKey::from_str("5fb92d6e98884f76de468fa3f6278f8807c48bebc13595d45af5bdc4da702133")
            .unwrap()
    }

    pub fn balthazar() -> SecretKey {
        SecretKey::from_str("8075991ce870b93a8870eca0c0f91913d12f47948ca0fd25b49c6fa7cdbeee8b")
            .unwrap()
    }
}
