use super::*;
use color_eyre::eyre;
use subxt::{DefaultConfig, DefaultExtra};

#[subxt::subxt(runtime_metadata_path = "metadata/canvas.scale")]
pub mod api {}

pub struct ContractsApi {
    api: api::RuntimeApi<DefaultConfig, DefaultExtra<DefaultConfig>>,
}

impl ContractsApi {
    pub fn new(client: subxt::Client<DefaultConfig>) -> Self {
        let api =
            client.to_runtime_api::<api::RuntimeApi<DefaultConfig, DefaultExtra<DefaultConfig>>>();
        Self { api }
    }

    /// Submit extrinsic to instantiate a contract with the given code.
    pub async fn instantiate_with_code(
        &self,
        value: Balance,
        gas_limit: Gas,
        storage_deposit_limit: Option<Balance>,
        code: Vec<u8>,
        data: Vec<u8>,
        salt: Vec<u8>,
        signer: &Signer,
    ) -> color_eyre::Result<AccountId> {
        let result = self
            .api
            .tx()
            .contracts()
            .instantiate_with_code(value, gas_limit, storage_deposit_limit, code, data, salt)
            .sign_and_submit_then_watch(signer)
            .await?
            .wait_for_in_block()
            .await?
            .wait_for_success()
            .await?;

        let instantiated = result
            .find_first::<api::contracts::events::Instantiated>()?
            .ok_or_else(|| eyre::eyre!("Failed to find Instantiated event"))?;

        Ok(instantiated.contract)
    }

    /// Submit extrinsic to call a contract.
    pub async fn call(
        &self,
        contract: AccountId,
        value: Balance,
        gas_limit: Gas,
        storage_deposit_limit: Option<Balance>,
        data: Vec<u8>,
        signer: &Signer,
    ) -> color_eyre::Result<Hash> {
        let tx_hash = self
            .api
            .tx()
            .contracts()
            .call(
                contract.into(),
                value,
                gas_limit,
                storage_deposit_limit,
                data,
            )
            .sign_and_submit(signer)
            .await?;

        Ok(tx_hash)
    }
}
