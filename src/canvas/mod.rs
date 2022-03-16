pub mod runner;
mod xts;

use crate::Cli;
use futures::{future, StreamExt};
use sp_core::sr25519;
use sp_keyring::AccountKeyring;
use subxt::{DefaultConfig, DefaultExtra, PairSigner};
use xts::ContractsApi;

pub type Balance = u128;
pub type Gas = u64;
pub type AccountId = <DefaultConfig as subxt::Config>::AccountId;
pub type Hash = <DefaultConfig as subxt::Config>::Hash;
pub type Signer = PairSigner<DefaultConfig, DefaultExtra<DefaultConfig>, sr25519::Pair>;

/// Trait implemented by [`smart_bench_macro::contract`] for all contract constructors.
pub trait InkConstructor: codec::Encode {
    const SELECTOR: [u8; 4];
}

/// Trait implemented by [`smart_bench_macro::contract`] for all contract messages.
pub trait InkMessage: codec::Encode {
    const SELECTOR: [u8; 4];
}

#[derive(clap::ArgEnum, Debug, Clone)]
pub enum Contract {
    All,
    Erc20,
    Flipper,
    Incrementer,
    Erc721,
    Erc1155,
}

smart_bench_macro::contract!("./contracts/erc20.contract");
smart_bench_macro::contract!("./contracts/flipper.contract");
smart_bench_macro::contract!("./contracts/incrementer.contract");
smart_bench_macro::contract!("./contracts/erc721.contract");
smart_bench_macro::contract!("./contracts/erc1155.contract");

pub async fn exec(cli: Cli) -> color_eyre::Result<()> {
    macro_rules! bench_contract {
        ($contract: path) => {
            matches!(&cli.contracts[..], &[Contract::All])
                || cli.contracts.iter().any(|c| matches!(c, $contract))
        };
    }

    let alice = PairSigner::new(AccountKeyring::Alice.pair());
    let bob = AccountKeyring::Bob.to_account_id();

    let mut runner = runner::BenchRunner::new(alice, cli.gas_limit, &cli.url).await?;

    // erc20
    if bench_contract!(Contract::Erc20) {
        let erc20_new = erc20::constructors::new(1_000_000);
        let erc20_transfer = || erc20::messages::transfer(bob.clone(), 1000).into();
        runner
            .prepare_contract("erc20", erc20_new, cli.instance_count, &erc20_transfer)
            .await?;
    }

    // flipper
    if bench_contract!(Contract::Flipper) {
        let flipper_new = flipper::constructors::new(false);
        let flipper_flip = || flipper::messages::flip().into();
        runner
            .prepare_contract("flipper", flipper_new, cli.instance_count, &flipper_flip)
            .await?;
    }

    // incrementer
    if bench_contract!(Contract::Incrementer) {
        let incrementer_new = incrementer::constructors::new(0);
        let incrementer_increment = || incrementer::messages::inc(1).into();
        runner
            .prepare_contract(
                "incrementer",
                incrementer_new,
                cli.instance_count,
                incrementer_increment,
            )
            .await?;
    }

    // erc721
    if bench_contract!(Contract::Erc721) {
        let erc721_new = erc721::constructors::new();
        let mut token_id = 0;
        let erc721_mint = || {
            let mint = erc721::messages::mint(token_id);
            token_id += 1;
            mint.into()
        };
        runner
            .prepare_contract("erc721", erc721_new, cli.instance_count, erc721_mint)
            .await?;
    }

    // erc1155
    if bench_contract!(Contract::Erc1155) {
        let erc1155_new = erc1155::constructors::new();
        let erc1155_create = || erc1155::messages::create(1_000_000).into();
        runner
            .prepare_contract("erc1155", erc1155_new, cli.instance_count, erc1155_create)
            .await?;
    }

    let result = runner.run(cli.call_count).await?;

    println!();
    result
        .for_each(|block| {
            println!(
                "Block {}, Extrinsics {}",
                block.block_number,
                block.extrinsics.len()
            );
            future::ready(())
        })
        .await;

    Ok(())
}