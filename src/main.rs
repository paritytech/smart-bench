mod canvas;
mod moonbeam;

// export for use by contract! macro
pub use canvas::{InkConstructor, InkMessage};
use clap::Parser;

#[derive(Debug, Parser)]
#[clap(version)]
pub struct Cli {
    /// the url of the substrate node for submitting the extrinsics.
    #[clap(name = "url", long, default_value = "ws://localhost:9944")]
    url: String,
    /// the chain to benchmark.
    #[clap(arg_enum)]
    chain: TargetChain,
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
pub enum TargetChain {
    Canvas,
    Moonbeam,
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

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    let cli = Cli::parse();
    tracing_subscriber::fmt::init();

    match cli.chain {
        TargetChain::Canvas => canvas::exec(cli).await,
        TargetChain::Moonbeam => moonbeam::exec(&cli).await,
    }
}
