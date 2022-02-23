use crate::canvas;
use color_eyre::{eyre, Result};
use sp_keyring::AccountKeyring;
use subxt::{PairSigner, Signer as _};
use subxt::rpc::RpcError::Call;

pub struct ContractBench {
    contract_name: &'static str,
    code: Vec<u8>,
    instantiate: ExtrinsicArgs,
    calls: Vec<ExtrinsicArgs>,
}

pub struct ExtrinsicArgs {
    value: canvas::Balance,
    gas_limit: Option<canvas::Balance>,
    storage_deposit_limit: Option<canvas::Balance>,
    data: Vec<u8>,
}

smart_bench_macro::contract!("./contracts/erc20.contract");
smart_bench_macro::contract!("./contracts/flipper.contract");

pub fn erc20_transfers(calls: u32, dest: &canvas::AccountId, amount: canvas::Balance) -> Result<ContractBench> {
    let code = load_contract("erc20")?;
    let instantiate = InstantiateArgs {

    };
    let calls = (0..calls).iter().map(|_| {
        CallArgs {

        }
    });

    Ok(ContractBench {
        contract_name: "erc20",
        code,
        instantiate,
        calls,
    })
}

fn load_contract(name: &str) -> Result<Vec<u8>> {
    let root = std::env::var("CARGO_MANIFEST_DIR")?;
    let contract_path = format!(name, "contracts/{}.contract");
    let metadata_path: std::path::PathBuf = [&root, contract_path].iter().collect();
    let reader = std::fs::File::open(metadata_path)?;
    let contract: contract_metadata::ContractMetadata = serde_json::from_reader(reader)?;
    let code = contract
        .source
        .wasm
        .ok_or_else(|| eyre::eyre!("contract bundle missing source Wasm"))?;
    Ok(code.0)
}