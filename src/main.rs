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
    color_eyre::install()?;

    let mut alice = PairSigner::new(AccountKeyring::Alice.pair());
    alice.set_nonce(0);
    let bob = AccountKeyring::Bob.to_account_id();

    let code =
        std::fs::read("/home/andrew/code/paritytech/ink/examples/erc20/target/ink/erc20.wasm")?;

    let instance_count = 30;
    let contract_accounts = erc20_instantiate(&mut alice, code, instance_count).await?;

    println!("Instantiated {} erc20 contracts", contract_accounts.len());

    let _block_subscription = canvas::BlocksSubscription::new().await?;

    let tx_hashes = erc20_transfer(&mut alice,&bob, 1, contract_accounts, 3).await?;

    println!("Submitted {} erc20 transfer calls", tx_hashes.len());

    async_std::task::sleep(std::time::Duration::from_secs(30)).await;

    Ok(())
}

async fn erc20_instantiate(
    signer: &mut canvas::Signer,
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
        signer.increment_nonce();
    }

    Ok(accounts)
}

async fn erc20_transfer(
    signer: &mut canvas::Signer,
    dest: &canvas::AccountId,
    amount: canvas::Balance,
    contracts: Vec<canvas::AccountId>,
    transfer_count: u32,
) -> color_eyre::Result<Vec<canvas::Hash>> {
    let gas_limit = 500_000_000_000;

    let transfer = erc20::messages::transfer(dest.clone(), amount);
    let mut tx_hashes = Vec::new();

    for contract in contracts {
        for _ in 0..transfer_count {
            let tx_hash = canvas::call(contract, 0, gas_limit, &transfer, &signer).await?;
            tx_hashes.push(tx_hash);
            signer.increment_nonce();
        }
    }

    Ok(tx_hashes)
}
