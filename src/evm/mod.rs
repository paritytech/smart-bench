mod runner;
mod transaction;
mod xts;

use crate::{
    evm::{runner::MoonbeamRunner, xts::MoonbeamApi},
    Cli, Contract,
};
use futures::{future, TryStreamExt};
use web3::{contract::tokens::Tokenize, signing::Key, types::U256};

pub async fn exec(cli: &Cli) -> color_eyre::Result<()> {
    let api = MoonbeamApi::new(&cli.url).await?;

    let mut runner = MoonbeamRunner::new(cli.url.to_string(), keyring::alith(), api);

    for contract in &cli.contracts {
        match contract {
            Contract::Erc20 => {
                let transfer_to = (&keyring::balthazar()).address();
                let ctor_params = (1_000_000u32,).into_tokens();
                let transfer_params = || (transfer_to, 1000u32).into_tokens();
                runner
                    .prepare_contract(
                        "BenchERC20",
                        cli.instance_count,
                        &ctor_params,
                        "transfer",
                        &transfer_params,
                    )
                    .await?;
            }
            Contract::Flipper => {
                let ctor_params = (true,).into_tokens();
                let flip_params = || Vec::new();
                runner
                    .prepare_contract(
                        "flipper",
                        cli.instance_count,
                        &ctor_params,
                        "flip",
                        &flip_params,
                    )
                    .await?;
            }
            Contract::Incrementer => {
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
            Contract::Erc721 => {
                let ctor_params = ().into_tokens();
                let mut token_id = 0;
                let mint_params = || {
                    let mint = (U256::from(token_id),).into_tokens();
                    token_id += 1;
                    mint
                };
                runner
                    .prepare_contract(
                        "BenchERC721",
                        cli.instance_count,
                        &ctor_params,
                        "mint",
                        mint_params,
                    )
                    .await?;
            }
            Contract::Erc1155 => {
                let ctor_params = ().into_tokens();
                let create_params = || (U256::from(1_000_000),).into_tokens();
                runner
                    .prepare_contract(
                        "BenchERC1155",
                        cli.instance_count,
                        &ctor_params,
                        "create",
                        create_params,
                    )
                    .await?;
            }
            Contract::OddProduct => {
                let ctor_params = ().into_tokens();
                let call_params = || (1000i32,).into_tokens();
                runner
                    .prepare_contract(
                        "Computation",
                        cli.instance_count,
                        &ctor_params,
                        "oddProduct",
                        call_params,
                    )
                    .await?;
            }
            Contract::TriangleNumber => {
                let ctor_params = ().into_tokens();
                let call_params = || (1000i32,).into_tokens();
                runner
                    .prepare_contract(
                        "Computation",
                        cli.instance_count,
                        &ctor_params,
                        "triangleNumber",
                        call_params,
                    )
                    .await?;
            }
        }
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
