pub mod runner;
mod xts;

use crate::{Cli, Contract, TargetPlatform};
use sp_core::sr25519;
use sp_keyring::AccountKeyring;
use subxt::{tx::PairSigner, utils::AccountId32, PolkadotConfig as DefaultConfig};
use xts::ContractsApi;

pub type Balance = u128;
pub type AccountId = <DefaultConfig as subxt::Config>::AccountId;
pub type Hash = <DefaultConfig as subxt::Config>::Hash;
pub type Signer = PairSigner<DefaultConfig, sr25519::Pair>;
pub type EventRecord = ();

pub const DERIVATION: &str = "//Sender/";

/// Trait implemented by [`smart_bench_macro::contract`] for all contract constructors.
pub trait InkConstructor: codec::Encode {
    const SELECTOR: [u8; 4];
}

/// Trait implemented by [`smart_bench_macro::contract`] for all contract messages.
pub trait InkMessage: codec::Encode {
    const SELECTOR: [u8; 4];
}

/// Solang compiled contracts to wasm generated API
mod solidity_contracts {
    smart_bench_macro::contract!("./contracts/solidity/wasm/BenchERC20.contract");
    smart_bench_macro::contract!("./contracts/solidity/wasm/flipper.contract");
    smart_bench_macro::contract!("./contracts/solidity/wasm/incrementer.contract");
    smart_bench_macro::contract!("./contracts/solidity/wasm/BenchERC721.contract");
    smart_bench_macro::contract!("./contracts/solidity/wasm/BenchERC1155.contract");
    smart_bench_macro::contract!("./contracts/solidity/wasm/Computation.contract");
    smart_bench_macro::contract!("./contracts/solidity/wasm/Storage.contract");
}

/// Ink contracts generated API
mod ink_contracts {
    smart_bench_macro::contract!("./contracts/ink/erc20.contract");
    smart_bench_macro::contract!("./contracts/ink/flipper.contract");
    smart_bench_macro::contract!("./contracts/ink/incrementer.contract");
    smart_bench_macro::contract!("./contracts/ink/erc721.contract");
    smart_bench_macro::contract!("./contracts/ink/erc1155.contract");
    smart_bench_macro::contract!("./contracts/ink/computation.contract");
    smart_bench_macro::contract!("./contracts/ink/storage.contract");
}

pub async fn exec(cli: Cli) -> color_eyre::Result<()> {
    let alice = PairSigner::new(AccountKeyring::Alice.pair());
    let bob: AccountId32 = AccountKeyring::Bob.to_account_id().into();
    let mut call_signers = None;

    if !cli.single_signer {
        call_signers = Some(vec![]);
        for i in 0..(cli.call_count * cli.instance_count * cli.contracts.len() as u32) {
            call_signers.as_mut().unwrap().push(generate_signer(i));
        }
    }

    let mut runner = runner::BenchRunner::new(alice, &cli.url, call_signers).await?;

    match cli.chain {
        TargetPlatform::SolWasm => prepare_solidity_contracts(&cli, &mut runner, bob).await?,
        TargetPlatform::InkWasm => prepare_ink_contracts(&cli, &mut runner, bob).await?,
        _ => panic!("Not supported target platform!"),
    }
    let result = runner.run(cli.call_count).await?;

    crate::print_block_info(result).await?;

    Ok(())
}

pub fn generate_signer(i: u32) -> Signer {
    let pair: sr25519::Pair = <sr25519::Pair as sp_core::Pair>::from_string(format!("{}{}", DERIVATION, i).as_str(), None).unwrap();
    let signer: Signer = PairSigner::new(pair);
    signer
}

pub async fn prepare_solidity_contracts(
    cli: &Cli,
    runner: &mut runner::BenchRunner,
    bob: AccountId32,
) -> color_eyre::Result<()> {
    use solidity_contracts::*;
    let path = "contracts/solidity/wasm";
    for contract in &cli.contracts {
        match contract {
            Contract::Erc20 => {
                let erc20_new = BenchERC20::constructors::new(1_000_000.into());
                let erc20_transfer =
                    || BenchERC20::messages::transfer(bob.clone(), 1000.into()).into();
                runner
                    .prepare_contract(
                        path,
                        "BenchERC20",
                        erc20_new,
                        cli.instance_count,
                        &erc20_transfer,
                    )
                    .await?;
            }
            Contract::Flipper => {
                let flipper_new = flipper::constructors::new(false);
                let flipper_flip = || flipper::messages::flip().into();
                runner
                    .prepare_contract(
                        path,
                        "flipper",
                        flipper_new,
                        cli.instance_count,
                        &flipper_flip,
                    )
                    .await?;
            }
            Contract::Incrementer => {
                let incrementer_new = incrementer::constructors::new(0);
                let incrementer_increment = || incrementer::messages::inc(1).into();
                runner
                    .prepare_contract(
                        path,
                        "incrementer",
                        incrementer_new,
                        cli.instance_count,
                        incrementer_increment,
                    )
                    .await?;
            }
            Contract::Erc721 => {
                let erc721_new = BenchERC721::constructors::new();
                let mut token_id = 0;
                let erc721_mint = || {
                    let mint = BenchERC721::messages::mint(token_id.into());
                    token_id += 1;
                    mint.into()
                };
                runner
                    .prepare_contract(
                        path,
                        "BenchERC721",
                        erc721_new,
                        cli.instance_count,
                        erc721_mint,
                    )
                    .await?;
            }
            Contract::Erc1155 => {
                let erc1155_new = BenchERC1155::constructors::new();
                let erc1155_create = || BenchERC1155::messages::create(1_000_000.into()).into();
                runner
                    .prepare_contract(
                        path,
                        "BenchERC1155",
                        erc1155_new,
                        cli.instance_count,
                        erc1155_create,
                    )
                    .await?;
            }
            Contract::OddProduct => {
                let computation_new = Computation::constructors::new();
                let computation_odd_product = || Computation::messages::oddProduct(1000).into();
                runner
                    .prepare_contract(
                        path,
                        "Computation",
                        computation_new,
                        cli.instance_count,
                        computation_odd_product,
                    )
                    .await?;
            }
            Contract::TriangleNumber => {
                let computation_new = Computation::constructors::new();
                let computation_triangle_number =
                    || Computation::messages::triangleNumber(1000).into();
                runner
                    .prepare_contract(
                        path,
                        "Computation",
                        computation_new,
                        cli.instance_count,
                        computation_triangle_number,
                    )
                    .await?;
            }
            Contract::StorageRead => {
                let storage_new = Storage::constructors::new();
                let storage_read = || Storage::messages::read(bob.clone(), 10).into();
                runner
                    .prepare_contract(
                        path,
                        "Storage",
                        storage_new,
                        cli.instance_count,
                        storage_read,
                    )
                    .await?;
            }
            Contract::StorageWrite => {
                let storage_new = Storage::constructors::new();
                let storage_read = || Storage::messages::write(bob.clone(), 10).into();
                runner
                    .prepare_contract(
                        path,
                        "Storage",
                        storage_new,
                        cli.instance_count,
                        storage_read,
                    )
                    .await?;
            }
            Contract::StorageReadWrite => {
                let storage_new = Storage::constructors::new();
                let storage_read = || Storage::messages::readWrite(bob.clone(), 10).into();
                runner
                    .prepare_contract(
                        path,
                        "Storage",
                        storage_new,
                        cli.instance_count,
                        storage_read,
                    )
                    .await?;
            }
        }
    }
    Ok(())
}

pub async fn prepare_ink_contracts(
    cli: &Cli,
    runner: &mut runner::BenchRunner,
    bob: AccountId32,
) -> color_eyre::Result<()> {
    use ink_contracts::*;
    let path = "contracts/ink";
    for contract in &cli.contracts {
        match contract {
            Contract::Erc20 => {
                let erc20_new = erc20::constructors::new(1_000_000);
                let erc20_transfer = || erc20::messages::transfer(bob.clone(), 1).into();
                runner
                    .prepare_contract(
                        path,
                        "erc20",
                        erc20_new,
                        cli.instance_count,
                        &erc20_transfer,
                    )
                    .await?;
            }
            Contract::Flipper => {
                let flipper_new = flipper::constructors::new(false);
                let flipper_flip = || flipper::messages::flip().into();
                runner
                    .prepare_contract(
                        path,
                        "flipper",
                        flipper_new,
                        cli.instance_count,
                        &flipper_flip,
                    )
                    .await?;
            }
            Contract::Incrementer => {
                let incrementer_new = incrementer::constructors::new(0);
                let incrementer_increment = || incrementer::messages::inc(1).into();
                runner
                    .prepare_contract(
                        path,
                        "incrementer",
                        incrementer_new,
                        cli.instance_count,
                        incrementer_increment,
                    )
                    .await?;
            }
            Contract::Erc721 => {
                let erc721_new = erc721::constructors::new();
                let mut token_id = 0;
                let erc721_mint = || {
                    let mint = erc721::messages::mint(token_id);
                    token_id += 1;
                    mint.into()
                };
                runner
                    .prepare_contract(path, "erc721", erc721_new, cli.instance_count, erc721_mint)
                    .await?;
            }
            Contract::Erc1155 => {
                let erc1155_new = erc1155::constructors::new();
                let erc1155_create = || erc1155::messages::create(1_000_000).into();
                runner
                    .prepare_contract(
                        path,
                        "erc1155",
                        erc1155_new,
                        cli.instance_count,
                        erc1155_create,
                    )
                    .await?;
            }
            Contract::OddProduct => {
                let computation_new = computation::constructors::new();
                let computation_odd_product = || computation::messages::odd_product(1000).into();
                runner
                    .prepare_contract(
                        path,
                        "computation",
                        computation_new,
                        cli.instance_count,
                        computation_odd_product,
                    )
                    .await?;
            }
            Contract::TriangleNumber => {
                let computation_new = computation::constructors::new();
                let computation_triangle_number =
                    || computation::messages::triangle_number(1000).into();
                runner
                    .prepare_contract(
                        path,
                        "computation",
                        computation_new,
                        cli.instance_count,
                        computation_triangle_number,
                    )
                    .await?;
            }
            Contract::StorageRead => {
                let storage_new = storage::constructors::new();
                let storage_read = || storage::messages::read(bob.clone(), 10).into();
                runner
                    .prepare_contract(
                        path,
                        "storage",
                        storage_new,
                        cli.instance_count,
                        storage_read,
                    )
                    .await?;
            }
            Contract::StorageWrite => {
                let storage_new = storage::constructors::new();
                let storage_read = || storage::messages::write(bob.clone(), 10).into();
                runner
                    .prepare_contract(
                        path,
                        "storage",
                        storage_new,
                        cli.instance_count,
                        storage_read,
                    )
                    .await?;
            }
            Contract::StorageReadWrite => {
                let storage_new = storage::constructors::new();
                let storage_read = || storage::messages::read_write(bob.clone(), 10).into();
                runner
                    .prepare_contract(
                        path,
                        "storage",
                        storage_new,
                        cli.instance_count,
                        storage_read,
                    )
                    .await?;
            }
        }
    }
    Ok(())
}
