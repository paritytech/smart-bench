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
smart_bench_macro::contract!("./contracts/incrementer.contract");
smart_bench_macro::contract!("./contracts/erc721.contract");
smart_bench_macro::contract!("./contracts/erc1155.contract");

#[async_std::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    let opts = Opts::from_args();

    let mut alice = PairSigner::new(AccountKeyring::Alice.pair());
    let bob = AccountKeyring::Bob.to_account_id();

    let client = subxt::ClientBuilder::new().build().await?;

    let alice_nonce = client
        .fetch_nonce::<canvas::api::DefaultAccountData>(alice.account_id())
        .await?;
    alice.set_nonce(alice_nonce);

    let api = canvas::ContractsApi::new(client);

    // erc20
    let erc20_new = erc20::constructors::new(1_000_000);
    let erc20_transfer = || erc20::messages::transfer(bob.clone(), 1000).into();
    let erc20_calls = prepare_contract(
        &api,
        "erc20",
        erc20_new,
        &mut alice,
        opts.instance_count,
        &erc20_transfer,
    )
    .await?;

    // flipper
    let flipper_new = flipper::constructors::new(false);
    let flipper_flip = || flipper::messages::flip().into();
    let flipper_calls = prepare_contract(
        &api,
        "flipper",
        flipper_new,
        &mut alice,
        opts.instance_count,
        &flipper_flip,
    )
    .await?;

    // incrementer
    let incrementer_new = incrementer::constructors::new(0);
    let incrementer_increment = || incrementer::messages::inc(1).into();
    let incrementer_calls = prepare_contract(
        &api,
        "incrementer",
        incrementer_new,
        &mut alice,
        opts.instance_count,
        incrementer_increment,
    )
    .await?;

    // erc721
    let erc721_new = erc721::constructors::new();
    let mut token_id = 0;
    let erc721_mint = || {
        let mint = erc721::messages::mint(token_id);
        token_id += 1;
        mint.into()
    };
    let erc721_calls = prepare_contract(
        &api,
        "erc721",
        erc721_new,
        &mut alice,
        opts.instance_count,
        erc721_mint,
    ).await?;

    // erc1155
    let erc1155_new = erc1155::constructors::new();
    let erc1155_create = || erc1155::messages::create(1_000_000).into();
    let erc1155_calls = prepare_contract(
        &api,
        "erc1155",
        erc1155_new,
        &mut alice,
        opts.instance_count,
        erc1155_create,
    ).await?;

    let all_contract_calls = vec![
        erc20_calls.iter().collect::<Vec<_>>(),
        flipper_calls.iter().collect::<Vec<_>>(),
        incrementer_calls.iter().collect::<Vec<_>>(),
        erc721_calls.iter().collect::<Vec<_>>(),
        erc1155_calls.iter().collect::<Vec<_>>(),
    ];

    let block_subscription = canvas::BlocksSubscription::new().await?;

    let mut tx_hashes = Vec::new();

    for _ in 0..opts.call_count {
        for i in 0..opts.instance_count {
            for contract_calls in &all_contract_calls {
                let contract_call = contract_calls
                    .get(i as usize)
                    .ok_or_else(|| eyre::eyre!("Missing contract instance at {}", i))?;
                let tx_hash = api
                    .call(
                        contract_call.contract_account.clone(),
                        0,
                        DEFAULT_GAS_LIMIT,
                        DEFAULT_STORAGE_DEPOSIT_LIMIT,
                        contract_call.call_data.0.clone(),
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

/// Upload and instantiate instances of contract, and build calls for benchmarking
async fn prepare_contract<C, F>(
    api: &canvas::ContractsApi,
    name: &str,
    constructor: C,
    signer: &mut canvas::Signer,
    instance_count: u32,
    mut create_message: F,
) -> color_eyre::Result<Vec<Call>>
where
    C: InkConstructor,
    F: FnMut() -> EncodedMessage,
{
    println!("Preparing {name}");

    let root = std::env::var("CARGO_MANIFEST_DIR")?;
    let contract_path = format!("contracts/{name}.contract");
    let metadata_path: std::path::PathBuf = [&root, &contract_path].iter().collect();
    let reader = std::fs::File::open(metadata_path)?;
    let contract: contract_metadata::ContractMetadata = serde_json::from_reader(reader)?;
    let code = contract
        .source
        .wasm
        .ok_or_else(|| eyre::eyre!("contract bundle missing source Wasm"))?;

    let contract_accounts =
        exec_instantiate(&api, signer, 0, code.0, &constructor, instance_count).await?;

    println!("Instantiated {} {name} contracts", contract_accounts.len());

    let calls = contract_accounts
        .iter()
        .map(|contract| {
            let message = create_message();
            Call { contract_account: contract.clone(), call_data: message }
        })
        .collect::<Vec<_>>();
    Ok(calls)
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
struct EncodedMessage(Vec<u8>);

impl EncodedMessage {
    fn new<M: InkMessage>(call: &M) -> Self {
        let mut call_data = M::SELECTOR.to_vec();
        <M as Encode>::encode_to(call, &mut call_data);
        Self(call_data)
    }
}

impl<M> From<M> for EncodedMessage where M: InkMessage
{
    fn from(msg: M) -> Self {
        EncodedMessage::new(&msg)
    }
}

#[derive(Clone)]
struct Call {
    contract_account: canvas::AccountId,
    call_data: EncodedMessage,
}
