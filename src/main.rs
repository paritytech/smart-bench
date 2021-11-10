mod canvas;

use sp_keyring::AccountKeyring;
use subxt::PairSigner;

/// Trait implemented by [`smart_bench_macro::contract`] for all contract constructors.
pub trait InkConstructor: codec::Encode {
    const SELECTOR: [u8; 4];
}

/// Trait implemented by [`smart_bench_macro::contract`] for all contract messages.
pub trait InkMessage: codec::Encode {
    const SELECTOR: [u8; 4];
}

smart_bench_macro::contract!("/home/andrew/code/paritytech/ink/examples/erc20");

#[async_std::main]
async fn main() -> color_eyre::Result<()> {
    let alice = PairSigner::new(AccountKeyring::Alice.pair());
    let bob = AccountKeyring::Bob.to_account_id();

    let code =
        std::fs::read("/home/andrew/code/paritytech/ink/examples/erc20/target/ink/erc20.wasm")?;

    let instance_count = 5;
    let contract_accounts = erc20_instantiate(&alice, code, instance_count).await?;

    println!("Instantiated {} erc20 contracts", contract_accounts.len());

    let tx_hashes = erc20_transfer(&alice,&bob, 1, contract_accounts).await?;

    println!("Submitted {} erc20 transfer calls", tx_hashes.len());

    Ok(())
}

async fn erc20_instantiate(
    signer: &canvas::Signer,
    code: Vec<u8>,
    count: u32,
) -> color_eyre::Result<Vec<canvas::AccountId>> {
    let endowment = 100_000_000_000_000_000;
    let gas_limit = 500_000_000_000;

    let initial_supply = 1_000_000;
    let constructor = erc20::constructors::new(initial_supply);

    let mut accounts = Vec::new();
    for i in 0..count {
        let salt = i.to_le_bytes().to_vec();
        let code = code.clone(); // subxt codegen generates constructor args by value atm

        let contract = canvas::instantiate_with_code(
            endowment,
            gas_limit,
            code.clone(),
            &constructor,
            salt,
            signer,
        )
        .await?;
        accounts.push(contract);
    }

    Ok(accounts)
}

async fn erc20_transfer(
    source: &canvas::Signer,
    dest: &canvas::AccountId,
    amount: canvas::Balance,
    contracts: Vec<canvas::AccountId>,
) -> color_eyre::Result<Vec<canvas::Hash>> {
    let gas_limit = 500_000_000_000;

    let transfer = erc20::messages::transfer(dest.clone(), amount);
    let mut tx_hashes = Vec::new();

    for contract in contracts {
        let tx_hash = canvas::call(contract, 0, gas_limit, &transfer, &source).await?;
        tx_hashes.push(tx_hash);
    }

    Ok(tx_hashes)
}
