mod canvas;
mod runner;

use codec::Encode;
use sp_keyring::AccountKeyring;
use structopt::StructOpt;
use subxt::PairSigner;

#[derive(Debug, StructOpt)]
pub struct Opts {
    /// the url of the substrate node for submitting the extrinsics.
    #[structopt(name = "url", long, default_value = "ws://localhost:9944")]
    url: String,
    /// The number of each contract to instantiate.
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

pub const DEFAULT_GAS_LIMIT: canvas::Gas = 500_000_000_000;
pub const DEFAULT_STORAGE_DEPOSIT_LIMIT: Option<canvas::Balance> = None;

smart_bench_macro::contract!("./contracts/erc20.contract");
smart_bench_macro::contract!("./contracts/flipper.contract");
smart_bench_macro::contract!("./contracts/incrementer.contract");
smart_bench_macro::contract!("./contracts/erc721.contract");
smart_bench_macro::contract!("./contracts/erc1155.contract");

#[async_std::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    let opts = Opts::from_args();

    let alice = PairSigner::new(AccountKeyring::Alice.pair());
    let bob = AccountKeyring::Bob.to_account_id();

    let mut runner = runner::BenchRunner::new(alice, &opts.url).await?;

    // erc20
    let erc20_new = erc20::constructors::new(1_000_000);
    let erc20_transfer = || erc20::messages::transfer(bob.clone(), 1000).into();
    runner
        .prepare_contract("erc20", erc20_new, opts.instance_count, &erc20_transfer)
        .await?;

    // flipper
    let flipper_new = flipper::constructors::new(false);
    let flipper_flip = || flipper::messages::flip().into();
    runner
        .prepare_contract("flipper", flipper_new, opts.instance_count, &flipper_flip)
        .await?;

    // incrementer
    let incrementer_new = incrementer::constructors::new(0);
    let incrementer_increment = || incrementer::messages::inc(1).into();
    runner
        .prepare_contract(
            "incrementer",
            incrementer_new,
            opts.instance_count,
            incrementer_increment,
        )
        .await?;

    // erc721
    let erc721_new = erc721::constructors::new();
    let mut token_id = 0;
    let erc721_mint = || {
        let mint = erc721::messages::mint(token_id);
        token_id += 1;
        mint.into()
    };
    runner
        .prepare_contract("erc721", erc721_new, opts.instance_count, erc721_mint)
        .await?;

    // erc1155
    let erc1155_new = erc1155::constructors::new();
    let erc1155_create = || erc1155::messages::create(1_000_000).into();
    runner
        .prepare_contract("erc1155", erc1155_new, opts.instance_count, erc1155_create)
        .await?;

    let result = runner.run(opts.call_count).await?;

    println!();
    for block in result.blocks {
        println!(
            "Block {}, Extrinsics {}",
            block.block_number,
            block.extrinsics.len()
        );
    }

    Ok(())
}

#[derive(Clone)]
pub struct EncodedMessage(Vec<u8>);

impl EncodedMessage {
    fn new<M: InkMessage>(call: &M) -> Self {
        let mut call_data = M::SELECTOR.to_vec();
        <M as Encode>::encode_to(call, &mut call_data);
        Self(call_data)
    }
}

impl<M> From<M> for EncodedMessage
where
    M: InkMessage,
{
    fn from(msg: M) -> Self {
        EncodedMessage::new(&msg)
    }
}

#[derive(Clone)]
pub struct Call {
    contract_account: canvas::AccountId,
    call_data: EncodedMessage,
}
