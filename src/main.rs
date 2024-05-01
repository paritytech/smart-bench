mod evm;
#[cfg(test)]
#[cfg(feature = "integration-tests")]
mod integration_tests;
mod stats;
mod wasm;

use std::fmt::Display;

// export for use by contract! macro
use clap::Parser;
pub use stats::{collect_block_stats, print_block_info, BlockInfo};
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
    InkWasm,
    SolWasm,
    Evm,
}

#[derive(clap::ArgEnum, Debug, Clone, Eq, PartialEq)]
pub enum Contract {
    Erc20,
    Flipper,
    Incrementer,
    Erc721,
    Erc1155,
    OddProduct,
    TriangleNumber,
    StorageRead,
    StorageWrite,
    StorageReadWrite,
}

impl Display for TargetPlatform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", clap::ArgEnum::to_possible_value(self).unwrap_or("unknown".into()).get_name())
    }
}

impl Display for Contract {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", clap::ArgEnum::to_possible_value(self).unwrap_or("unknown".into()).get_name())
    }
}

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    let cli = Cli::parse();
    tracing_subscriber::fmt::init();

    println!("Smart-bench run parameters:");
    println!("Platform: {}", cli.chain);
    println!("Contracts: {}", cli.contracts.iter().map(|arg| arg.to_string()).collect::<Vec<_>>().join("+"));

    match cli.chain {
        TargetPlatform::InkWasm => wasm::exec(cli).await,
        TargetPlatform::SolWasm => wasm::exec(cli).await,
        TargetPlatform::Evm => evm::exec(&cli).await,
    }
}
