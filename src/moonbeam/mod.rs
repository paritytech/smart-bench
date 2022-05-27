mod runner;
mod transaction;
mod xts;

use crate::{
    moonbeam::{runner::MoonbeamRunner, xts::MoonbeamApi},
    Cli, Contract,
};
use futures::{future, TryStreamExt};
use web3::contract::tokens::Tokenize;

pub async fn exec(cli: &Cli) -> color_eyre::Result<()> {
    let api = MoonbeamApi::new(&cli.url).await?;

    let mut runner = MoonbeamRunner::new(cli.url.to_string(), keyring::alith(), api);

    if cli.should_bench_contract(Contract::Incrementer) {
        let ctor_params = (1u32,).into_tokens();
        let inc_params = || (1u32,).into_tokens();
        runner
            .prepare_contract(
                "incrementer",
                cli.instance_count,
                &ctor_params,
                "inc",
                inc_params,
            )
            .await?;
    }

    let result = runner.run(cli.call_count).await?;

    println!();
    result
        .try_for_each(|block| {
            println!("{}", block.stats);
            future::ready(Ok(()))
        })
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
