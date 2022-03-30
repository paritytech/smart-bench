use super::*;
use jsonrpsee::{
    core::client::ClientT,
    rpc_params,
    ws_client::{WsClient, WsClientBuilder},
};
use serde::Serialize;
use sp_core::{Bytes, H256};
use subxt::{rpc::NumberOrHex, DefaultConfig, DefaultExtra, Signer as _};

pub struct ContractsApi {
    pub api: api::RuntimeApi<DefaultConfig, DefaultExtra<DefaultConfig>>,
    ws_client: WsClient,
}

impl ContractsApi {
    pub async fn new(client: subxt::Client<DefaultConfig>, url: &str) -> color_eyre::Result<Self> {
        let api =
            client.to_runtime_api::<api::RuntimeApi<DefaultConfig, DefaultExtra<DefaultConfig>>>();
        let ws_client = WsClientBuilder::default().build(&url).await?;
        Ok(Self { api, ws_client })
    }

    /// Submit extrinsic to instantiate a contract with the given code.
    pub async fn instantiate_with_code_dry_run(
        &self,
        value: Balance,
        gas_limit: Gas,
        storage_deposit_limit: Option<Balance>,
        code: Vec<u8>,
        data: Vec<u8>,
        salt: Vec<u8>,
        signer: &Signer,
    ) -> color_eyre::Result<ContractInstantiateResult> {
        let storage_deposit_limit = storage_deposit_limit.map(|n| NumberOrHex::Hex(n.into()));
        let code = Code::Upload(code.into());
        let call_request = InstantiateRequest {
            origin: signer.account_id().clone(),
            value: NumberOrHex::Hex(value.into()),
            gas_limit: NumberOrHex::Number(gas_limit),
            storage_deposit_limit,
            code,
            data: data.into(),
            salt: salt.into(),
        };
        let params = rpc_params![call_request];
        let result: ContractInstantiateResult = self
            .ws_client
            .request("contracts_instantiate", params)
            .await?;
        Ok(result)
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
    ) -> color_eyre::Result<H256> {
        let tx_hash = self
            .api
            .tx()
            .contracts()
            .instantiate_with_code(value, gas_limit, storage_deposit_limit, code, data, salt)
            .sign_and_submit(signer)
            .await?;

        Ok(tx_hash)
    }

    /// Submit extrinsic to call a contract.
    pub async fn call_dry_run(
        &self,
        contract: AccountId,
        value: Balance,
        gas_limit: Gas,
        storage_deposit_limit: Option<Balance>,
        data: Vec<u8>,
        signer: &Signer,
    ) -> color_eyre::Result<ContractExecResult> {
        let storage_deposit_limit = storage_deposit_limit.map(|n| NumberOrHex::Hex(n.into()));
        let call_request = RpcCallRequest {
            origin: signer.account_id().clone(),
            dest: contract,
            value: NumberOrHex::Hex(value.into()),
            gas_limit: NumberOrHex::Number(gas_limit.into()),
            storage_deposit_limit,
            input_data: Bytes(data),
        };
        let params = rpc_params![call_request];
        let result: ContractExecResult = self.ws_client.request("contracts_call", params).await?;
        Ok(result)
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
type ContractExecResult = pallet_contracts_primitives::ContractExecResult<Balance>;

type ContractInstantiateResult =
    pallet_contracts_primitives::ContractInstantiateResult<AccountId, Balance>;

/// A struct that encodes RPC parameters required to instantiate a new smart contract.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct InstantiateRequest {
    origin: AccountId,
    value: NumberOrHex,
    gas_limit: NumberOrHex,
    storage_deposit_limit: Option<NumberOrHex>,
    code: Code,
    data: Bytes,
    salt: Bytes,
}

/// Reference to an existing code hash or a new Wasm module.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
enum Code {
    /// A Wasm module as raw bytes.
    Upload(Bytes),
    #[allow(unused)]
    /// The code hash of an on-chain Wasm blob.
    Existing(H256),
}

/// A struct that encodes RPC parameters required for a call to a smart contract.
///
/// Copied from `pallet-contracts-rpc`.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RpcCallRequest {
    origin: AccountId,
    dest: AccountId,
    value: NumberOrHex,
    gas_limit: NumberOrHex,
    storage_deposit_limit: Option<NumberOrHex>,
    input_data: Bytes,
}
