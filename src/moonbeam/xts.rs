use super::account::{AccountId20, EthereumSignature};
use color_eyre::eyre;
use sp_core::{ecdsa, H160, H256, U256};
use subxt::{PolkadotExtrinsicParams, PairSigner};

pub enum MoonbeamConfig {}

impl subxt::Config for MoonbeamConfig {
    type Index = u32;
    type BlockNumber = u32;
    type Hash = sp_core::H256;
    type Hashing = sp_runtime::traits::BlakeTwo256;
    type AccountId = AccountId20;
    type Address = Self::AccountId;
    type Header = sp_runtime::generic::Header<Self::BlockNumber, sp_runtime::traits::BlakeTwo256>;
    type Signature = EthereumSignature;
    type Extrinsic = sp_runtime::OpaqueExtrinsic;
}

pub type Signer = PairSigner<MoonbeamConfig, ecdsa::Pair>;

#[subxt::subxt(runtime_metadata_path = "metadata/moonbeam.scale")]
pub mod api {
    #[subxt(substitute_type = "primitive_types::H160")]
    use sp_core::H160;
    #[subxt(substitute_type = "primitive_types::U256")]
    use sp_core::U256;
    #[subxt(substitute_type = "account::AccountId20")]
    use crate::moonbeam::xts::AccountId20;
}

pub struct MoonbeamApi {
    api: api::RuntimeApi<MoonbeamConfig, PolkadotExtrinsicParams<MoonbeamConfig>>,
}

impl MoonbeamApi {
    pub fn new(client: subxt::Client<MoonbeamConfig>) -> Self {
        let api = client
            .to_runtime_api::<api::RuntimeApi<MoonbeamConfig, PolkadotExtrinsicParams<MoonbeamConfig>>>();
        Self { api }
    }

    pub async fn transfer(
        &self,
        signer: &Signer,
        dest: AccountId20,
    ) -> color_eyre::Result<()> {
        let result = self
            .api
            .tx()
            .balances()
            .transfer(
                dest,
                10_000
            )?
            .sign_and_submit_then_watch_default(signer)
            .await?
            .wait_for_in_block()
            .await?
            .wait_for_success()
            .await?;

        let _ = result
            .find_first::<api::balances::events::Transfer>()?
            .ok_or_else(|| eyre::eyre!("Failed to find Transfer event"))?;

        Ok(())
    }


    pub async fn create2(
        &self,
        data: Vec<u8>,
        salt: H256,
        value: U256,
        gas_limit: u64,
        nonce: Option<U256>,
        signer: &Signer,
    ) -> color_eyre::Result<AccountId20> {
        let from = H160(signer.account_id().0);
        let max_fee_per_gas = U256::max_value();
        let max_priority_fee_per_gas = None;
        let access_list = Vec::new();
        let result = self
            .api
            .tx()
            .evm()
            .create2(
                from,
                data,
                salt,
                value,
                gas_limit,
                max_fee_per_gas,
                nonce,
                max_priority_fee_per_gas,
                access_list,
            )?
            .sign_and_submit_then_watch_default(signer)
            .await?
            .wait_for_in_block()
            .await?
            .wait_for_success()
            .await?;

        let created = result
            .find_first::<api::evm::events::Created>()?
            .ok_or_else(|| eyre::eyre!("Failed to find Created event"))?;

        Ok(AccountId20(created.0 .0))
    }

    pub async fn call(
        &self,
        source: H160,
        target: H160,
        input: Vec<u8>,
        value: U256,
        gas_limit: u64,
        nonce: Option<U256>,
        signer: &Signer,
    ) -> color_eyre::Result<H256> {
        let max_fee_per_gas = U256::max_value();
        let max_priority_fee_per_gas = None;
        let access_list = Vec::new();
        let tx_hash = self
            .api
            .tx()
            .evm()
            .call(
                source,
                target,
                input,
                value,
                gas_limit,
                max_fee_per_gas,
                nonce,
                max_priority_fee_per_gas,
                access_list,
            )?
            .sign_and_submit_default(signer)
            .await?;

        Ok(tx_hash)
    }
}

pub fn alice() -> Signer {
    let pair = <ecdsa::Pair as sp_core::Pair>::from_string(
        "0x5fb92d6e98884f76de468fa3f6278f8807c48bebc13595d45af5bdc4da702133",
        None,
    )
    .unwrap();
    Signer::new(pair)
}

pub fn bob() -> Signer {
    let pair = <ecdsa::Pair as sp_core::Pair>::from_string(
        "0x8075991ce870b93a8870eca0c0f91913d12f47948ca0fd25b49c6fa7cdbeee8b",
        None,
    )
        .unwrap();
    Signer::new(pair)
}
