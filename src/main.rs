mod exec;

use color_eyre::eyre;
use sp_core::sr25519;
use subxt::PairSigner;

#[subxt::subxt(runtime_metadata_path = "metadata/contracts_runtime.scale")]
pub mod canvas {}

smart_bench_macro::contract!("/home/andrew/code/paritytech/ink/examples/erc20");

type Balance = u128;
type Gas = u64;
type ContractAccount = <canvas::DefaultConfig as subxt::Config>::AccountId;
type Hash = <canvas::DefaultConfig as subxt::Config>::Hash;
type Signer = PairSigner<canvas::DefaultConfig, sr25519::Pair>;

async fn api() -> color_eyre::Result<canvas::RuntimeApi<canvas::DefaultConfig>> {
    Ok(subxt::ClientBuilder::new()
        // .set_url()
        .build()
        .await?
        .to_runtime_api::<canvas::RuntimeApi<canvas::DefaultConfig>>())
}

#[async_std::main]
async fn main() -> color_eyre::Result<()> {
    println!("Hello, world!");
    Ok(())
}
