mod canvas;
mod contracts;

use color_eyre::eyre;
use contracts::erc20;
use sp_keyring::AccountKeyring;
use structopt::StructOpt;
use subxt::{PairSigner, Signer as _};

#[derive(Debug, StructOpt)]
pub struct Opts {
    /// The number of contracts to instantiate.
    #[structopt(long, short)]
    instance_count: u32,
    /// The number of calls to make to each contract.
    #[structopt(long, short)]
    call_count: u32,
}

/// Trait implemented by [`smart_bench_macro::contract`] for all contract constructors.
pub trait InkConstructor: codec::Encode {
    const SELECTOR: [u8; 4];
}

/// Trait implemented by [`smart_bench_macro::contract`] for all contract messages.
pub trait InkMessage: codec::Encode {
    const SELECTOR: [u8; 4];
}

#[async_std::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    let opts = Opts::from_args();

    let mut alice = PairSigner::new(AccountKeyring::Alice.pair());

    let client = subxt::ClientBuilder::new().build().await?;

    let alice_nonce = client
        .fetch_nonce::<canvas::api::DefaultAccountData>(alice.account_id())
        .await?;
    alice.set_nonce(alice_nonce);

    let bob = AccountKeyring::Bob.to_account_id();

    let root = std::env::var("CARGO_MANIFEST_DIR")?;
    let contract_path = "contracts/erc20.contract";
    let metadata_path: std::path::PathBuf = [&root, contract_path].iter().collect();
    let reader = std::fs::File::open(metadata_path)?;
    let contract: contract_metadata::ContractMetadata = serde_json::from_reader(reader)?;

    let code = contract
        .source
        .wasm
        .ok_or_else(|| eyre::eyre!("contract bundle missing source Wasm"))?;

    let api = canvas::ContractsApi::new(client);

    let contract_accounts = erc20::new(&api, &mut alice, code.0, opts.instance_count).await?;

    println!("Instantiated {} erc20 contracts", contract_accounts.len());

    let block_subscription = canvas::BlocksSubscription::new().await?;

    let tx_hashes = erc20::transfer(
        &api,
        &mut alice,
        &bob,
        1,
        contract_accounts,
        opts.call_count,
    )
    .await?;

    println!("Submitted {} erc20 transfer calls", tx_hashes.len());

    let result = block_subscription.wait_for_txs(&tx_hashes).await?;

    for block in result.blocks {
        println!(
            "Block {}, Extrinsics {}",
            block.block_number,
            block.extrinsics.len()
        );
    }

    Ok(())
}
