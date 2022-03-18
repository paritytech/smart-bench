mod canvas;

// export for use by contract! macro
pub use canvas::{InkConstructor, InkMessage};

use canvas::Contract as WasmContract;
use clap::Parser;

#[derive(Debug, Parser)]
#[clap(version)]
pub struct Cli {
    /// the url of the substrate node for submitting the extrinsics.
    #[clap(name = "url", long, default_value = "ws://localhost:9944")]
    url: String,
    /// the list of contracts to benchmark with.
    #[clap(arg_enum)]
    contracts: Vec<WasmContract>,
    /// the number of each contract to instantiate.
    #[clap(long, short)]
    instance_count: u32,
    /// the number of calls to make to each contract.
    #[clap(long, short)]
    call_count: u32,
    /// gas limit for all contract extrinsics.
    #[clap(long, short, default_value = "50000000000")]
    gas_limit: canvas::Gas,
}

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    let cli = Cli::parse();

    canvas::exec(cli).await?;

    Ok(())
}
