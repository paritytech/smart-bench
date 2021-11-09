mod canvas;

use color_eyre::eyre;
use sp_core::sr25519;
use subxt::PairSigner;

smart_bench_macro::contract!("/home/andrew/code/paritytech/ink/examples/erc20");

#[async_std::main]
async fn main() -> color_eyre::Result<()> {
    println!("Hello, world!");
    Ok(())
}
