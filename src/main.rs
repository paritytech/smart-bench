mod evm;
mod wasm;

// export for use by contract! macro
use clap::Parser;
pub use wasm::{InkConstructor, InkMessage};

#[derive(Debug, Parser)]
#[clap(version)]
pub struct Cli {
    /// the url of the substrate node for submitting the extrinsics.
    #[clap(name = "url", long, default_value = "ws://localhost:9944")]
    url: String,
    /// the smart contract platform to benchmark.
    #[clap(arg_enum)]
    chain: TargetPlatform,
    /// the list of contracts to benchmark with.
    #[clap(arg_enum)]
    contracts: Vec<Contract>,
    /// the number of each contract to instantiate.
    #[clap(long, short)]
    instance_count: u32,
    /// the number of calls to make to each contract.
    #[clap(long, short)]
    call_count: u32,
}

#[derive(clap::ArgEnum, Debug, Clone)]
pub enum TargetPlatform {
    Wasm,
    Evm,
}

#[derive(clap::ArgEnum, Debug, Clone, Eq, PartialEq)]
pub enum Contract {
    Erc20,
    Flipper,
    Incrementer,
    Erc721,
    Erc1155,
}

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    let cli = Cli::parse();
    tracing_subscriber::fmt::init();

    match cli.chain {
        TargetPlatform::Wasm => wasm::exec(cli).await,
        TargetPlatform::Evm => evm::exec(&cli).await,
    }
}
