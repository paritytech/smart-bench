use crate::canvas;

smart_bench_macro::contract!("./contracts/erc20.contract");

pub async fn new(
    api: &canvas::ContractsApi,
    signer: &mut canvas::Signer,
    code: Vec<u8>,
    count: u32,
) -> color_eyre::Result<Vec<canvas::AccountId>> {
    let value = 0;
    let gas_limit = 500_000_000_000;
    let storage_deposit_limit = None;

    let initial_supply = 1_000_000;
    let constructor = erc20::constructors::new(initial_supply);

    let mut accounts = Vec::new();
    for i in 0..count {
        let salt = i.to_le_bytes().to_vec();
        let code = code.clone(); // subxt codegen generates constructor args by value atm

        let contract = api
            .instantiate_with_code(
                value,
                gas_limit,
                storage_deposit_limit,
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

pub async fn transfer(
    api: &canvas::ContractsApi,
    signer: &mut canvas::Signer,
    dest: &canvas::AccountId,
    amount: canvas::Balance,
    contracts: Vec<canvas::AccountId>,
    transfer_count: u32,
) -> color_eyre::Result<Vec<canvas::Hash>> {
    let gas_limit = 500_000_000_000;
    let storage_deposit_limit: Option<canvas::Balance> = None;

    let transfer = erc20::messages::transfer(dest.clone(), amount);
    let mut tx_hashes = Vec::new();

    for contract in contracts {
        for _ in 0..transfer_count {
            let tx_hash = api
                .call(
                    contract.clone(),
                    0,
                    gas_limit,
                    storage_deposit_limit,
                    &transfer,
                    signer,
                )
                .await?;
            tx_hashes.push(tx_hash);
            signer.increment_nonce();
        }
    }

    Ok(tx_hashes)
}
