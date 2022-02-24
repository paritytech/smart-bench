mod canvas;

use codec::Encode;
use color_eyre::eyre;
use sp_keyring::AccountKeyring;
use structopt::StructOpt;
use subxt::{PairSigner, Signer as _};

#[derive(Debug, StructOpt)]
pub struct Opts {
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

const DEFAULT_GAS_LIMIT: canvas::Gas = 500_000_000_000;
const DEFAULT_STORAGE_DEPOSIT_LIMIT: Option<canvas::Balance> = None;

smart_bench_macro::contract!("./contracts/erc20.contract");
smart_bench_macro::contract!("./contracts/flipper.contract");

#[async_std::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    let opts = Opts::from_args();

    let mut alice = PairSigner::new(AccountKeyring::Alice.pair());

    let client = subxt::ClientBuilder::new().build().await?;

    let alice_nonce = client
        .fetch_nonce::<canvas::api::DefaultAccountData>(alice.account_id())
        .await?;
    alice.set_nonce(alice_nonce);

    let api = canvas::ContractsApi::new(client);

    let bob = AccountKeyring::Bob.to_account_id();

    // erc20
    let erc20_code = load_contract("erc20")?;
    let initial_supply = 1_000_000;
    let erc20_new = erc20::constructors::new(initial_supply);

    let erc20_contracts = exec_instantiate(
        &api,
        &mut alice,
        0,
        erc20_code,
        &erc20_new,
        opts.instance_count,
    )
    .await?;

    println!("Instantiated {} erc20 contracts", erc20_contracts.len());

    let erc20_calls = erc20_contracts
        .iter()
        .map(|contract| {
            let transfer = erc20::messages::transfer(bob.clone(), 1000);
            Call::new(contract.clone(), &transfer)
        })
        .collect::<Vec<_>>();

    // flipper
    let flipper_code = load_contract("flipper")?;
    let flipper_new = flipper::constructors::new(false);

    let flipper_contracts = exec_instantiate(
        &api,
        &mut alice,
        0,
        flipper_code,
        &flipper_new,
        opts.instance_count,
    )
    .await?;

    println!("Instantiated {} flipper contracts", flipper_contracts.len());

    let flipper_calls = flipper_contracts
        .iter()
        .map(|contract| {
            let flip = flipper::messages::flip();
            Call::new(contract.clone(), &flip)
        })
        .collect::<Vec<_>>();

    let all_contract_calls = vec![
        erc20_calls.iter().collect::<Vec<_>>(),
        flipper_calls.iter().collect::<Vec<_>>(),
    ];

    let block_subscription = canvas::BlocksSubscription::new().await?;

    let mut tx_hashes = Vec::new();

    for _ in 0..opts.call_count {
        for i in 0..opts.instance_count {
            for contract_calls in all_contract_calls {
                let contract_call = contract_calls
                    .get(i as usize)
                    .ok_or_else(|| eyre::eyre!("Missing contract instance at {}", i))?;
                let tx_hash = api
                    .call(
                        contract_call.contract_account.clone(),
                        0,
                        DEFAULT_GAS_LIMIT,
                        DEFAULT_STORAGE_DEPOSIT_LIMIT,
                        contract_call.call_data.clone(),
                        &alice,
                    )
                    .await?;
                alice.increment_nonce();
                tx_hashes.push(tx_hash)
            }
        }
    }

    println!("Submitted {} total contract calls", tx_hashes.len());

    let result = block_subscription.wait_for_txs(&tx_hashes).await?;

    for block in result.blocks {
        println!(
            "Block {}, Extrinsics {}",
            block.block_number,
            block.extrinsics.len()
        );
    }

    Ok(())
}

fn load_contract(name: &str) -> color_eyre::Result<Vec<u8>> {
    let root = std::env::var("CARGO_MANIFEST_DIR")?;
    let contract_path = format!("contracts/{name}.contract");
    let metadata_path: std::path::PathBuf = [&root, &contract_path].iter().collect();
    let reader = std::fs::File::open(metadata_path)?;
    let contract: contract_metadata::ContractMetadata = serde_json::from_reader(reader)?;
    let code = contract
        .source
        .wasm
        .ok_or_else(|| eyre::eyre!("contract bundle missing source Wasm"))?;
    Ok(code.0)
}

async fn exec_instantiate<C: InkConstructor>(
    api: &canvas::ContractsApi,
    signer: &mut canvas::Signer,
    value: canvas::Balance,
    code: Vec<u8>,
    constructor: &C,
    count: u32,
) -> color_eyre::Result<Vec<canvas::AccountId>> {
    let mut data = C::SELECTOR.to_vec();
    <C as Encode>::encode_to(constructor, &mut data);

    let mut accounts = Vec::new();
    for i in 0..count {
        let salt = i.to_le_bytes().to_vec();

        let contract = api
            .instantiate_with_code(
                value,
                DEFAULT_GAS_LIMIT,
                DEFAULT_STORAGE_DEPOSIT_LIMIT,
                code.clone(),
                data.clone(),
                salt,
                signer,
            )
            .await?;
        accounts.push(contract);
        signer.increment_nonce();
    }

    Ok(accounts)
}

#[derive(Clone)]
struct Call {
    contract_account: canvas::AccountId,
    call_data: Vec<u8>,
}

impl Call {
    fn new<M: InkMessage>(contract_account: canvas::AccountId, call: &M) -> Self {
        let mut call_data = M::SELECTOR.to_vec();
        <M as Encode>::encode_to(call, &mut call_data);

        Self {
            contract_account,
            call_data,
        }
    }
}
