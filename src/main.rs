mod canvas;

use sp_keyring::AccountKeyring;
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
    let initial_supply = 1_000_000;
    let constructor = erc20::constructors::new(initial_supply);

    let endowment = 100_000_000_000_000_000;
    let gas_limit = 500_000_000_000;
    let salt = vec![];
    let signer = PairSigner::new(AccountKeyring::Alice.pair());

    let code = std::fs::read("/home/andrew/code/paritytech/ink/examples/erc20/target/ink/erc20.wasm")?;

    let contract = canvas::instantiate_with_code(endowment, gas_limit, code, constructor, salt, &signer).await?;

    println!("CONTRACT {}", contract);

    Ok(())
}
