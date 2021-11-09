mod canvas;

use color_eyre::eyre;
use sp_core::sr25519;
use subxt::PairSigner;

pub trait InkConstructor: codec::Encode {
    const SELECTOR: [u8; 4];
}

pub trait InkMessage: codec::Encode {
    const SELECTOR: [u8; 4];
}

smart_bench_macro::contract!("/home/andrew/code/paritytech/ink/examples/erc20");

#[async_std::main]
async fn main() -> color_eyre::Result<()> {
    println!("Hello, world!");
    Ok(())
}
