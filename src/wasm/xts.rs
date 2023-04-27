use super::*;
use codec::{Decode, Encode, MaxEncodedLen};
use pallet_contracts_primitives::{ContractExecResult, ContractInstantiateResult};
use serde::{Deserialize, Serialize};
use sp_core::{Bytes, H256};
use subxt::{
    ext::scale_encode::EncodeAsType, rpc_params, utils::MultiAddress, OnlineClient,
    PolkadotConfig as DefaultConfig,
};

const DRY_RUN_GAS_LIMIT: Option<Weight> = None;

#[subxt::subxt(runtime_metadata_path = "metadata/contracts-node.scale")]
pub mod api {}

pub struct ContractsApi {
    pub client: OnlineClient<DefaultConfig>,
}

impl ContractsApi {
    pub async fn new(client: OnlineClient<DefaultConfig>) -> color_eyre::Result<Self> {
        Ok(Self { client })
    }

    /// Submit extrinsic to instantiate a contract with the given code.
    pub async fn instantiate_with_code_dry_run(
        &self,
        value: Balance,
        storage_deposit_limit: Option<Balance>,
        code: Vec<u8>,
        data: Vec<u8>,
        salt: Vec<u8>,
        signer: &Signer,
    ) -> ContractInstantiateResult<AccountId, Balance> {
        let code = Code::Upload(code);
        let call_request = InstantiateRequest {
            origin: subxt::tx::Signer::account_id(signer).clone(),
            value,
            gas_limit: DRY_RUN_GAS_LIMIT,
            storage_deposit_limit,
            code,
            data,
            salt,
        };
        let func = "ContractsApi_instantiate";
        let params = rpc_params![func, Bytes(Encode::encode(&call_request))];
        let bytes: Bytes = self
            .client
            .rpc()
            .request("state_call", params)
            .await
            .unwrap_or_else(|err| {
                panic!("error on ws request `contracts_instantiate`: {err:?}");
            });
        Decode::decode(&mut bytes.as_ref())
            .unwrap_or_else(|err| panic!("decoding ContractInstantiateResult failed: {err}"))
    }

    /// Submit extrinsic to instantiate a contract with the given code.
    pub async fn instantiate_with_code(
        &self,
        value: Balance,
        gas_limit: Weight,
        storage_deposit_limit: Option<Balance>,
        code: Vec<u8>,
        data: Vec<u8>,
        salt: Vec<u8>,
        signer: &Signer,
    ) -> color_eyre::Result<H256> {
        let call = subxt::tx::Payload::new(
            "Contracts",
            "instantiate_with_code",
            InstantiateWithCode {
                value,
                gas_limit,
                storage_deposit_limit,
                code,
                data,
                salt,
            },
        )
        .unvalidated();

        let tx_hash = self
            .client
            .tx()
            .sign_and_submit_default(&call, signer)
            .await?;

        Ok(tx_hash)
    }

    /// Submit extrinsic to call a contract.
    pub async fn call_dry_run(
        &self,
        contract: AccountId,
        value: Balance,
        storage_deposit_limit: Option<Balance>,
        input_data: Vec<u8>,
        signer: &Signer,
    ) -> color_eyre::Result<ContractExecResult<Balance>> {
        let call_request = RpcCallRequest {
            origin: signer.account_id().clone(),
            dest: contract,
            value,
            gas_limit: DRY_RUN_GAS_LIMIT,
            storage_deposit_limit,
            input_data,
        };
        let params = rpc_params!["ContractsApi_call", Bytes(Encode::encode(&call_request))];
        let bytes: Bytes = self
            .client
            .rpc()
            .request("state_call", params)
            .await
            .unwrap_or_else(|err| {
                panic!("error on ws request `contracts_call`: {err:?}");
            });
        let result: ContractExecResult<Balance> = Decode::decode(&mut bytes.as_ref())
            .unwrap_or_else(|err| panic!("decoding ContractExecResult failed: {err}"));

        Ok(result)
    }

    /// Submit extrinsic to call a contract.
    pub async fn call(
        &self,
        contract: AccountId,
        value: Balance,
        gas_limit: Weight,
        storage_deposit_limit: Option<Balance>,
        data: Vec<u8>,
        signer: &Signer,
    ) -> color_eyre::Result<Hash> {
        let call = subxt::tx::Payload::new(
            "Contracts",
            "call",
            Call {
                dest: contract.into(),
                value,
                gas_limit,
                storage_deposit_limit,
                data,
            },
        )
        .unvalidated();

        let tx_hash = self
            .client
            .tx()
            .sign_and_submit_default(&call, signer)
            .await?;

        Ok(tx_hash)
    }
}

/// A raw call to `pallet-contracts`'s `call`.
#[derive(Debug, Decode, Encode, EncodeAsType)]
#[encode_as_type(trait_bounds = "", crate_path = "subxt::ext::scale_encode")]
pub struct Call {
    dest: MultiAddress<AccountId, ()>,
    #[codec(compact)]
    value: Balance,
    gas_limit: Weight,
    storage_deposit_limit: Option<Balance>,
    data: Vec<u8>,
}

/// A raw call to `pallet-contracts`'s `instantiate_with_code`.
#[derive(Debug, Encode, Decode, EncodeAsType)]
#[encode_as_type(trait_bounds = "", crate_path = "subxt::ext::scale_encode")]
pub struct InstantiateWithCode {
    #[codec(compact)]
    value: Balance,
    gas_limit: Weight,
    storage_deposit_limit: Option<Balance>,
    code: Vec<u8>,
    data: Vec<u8>,
    salt: Vec<u8>,
}

/// Copied from `sp_weight` to additionally implement `scale_encode::EncodeAsType`.
#[derive(
    Copy,
    Clone,
    Eq,
    PartialEq,
    Debug,
    Default,
    Encode,
    Decode,
    MaxEncodedLen,
    EncodeAsType,
    Serialize,
    Deserialize,
)]
#[encode_as_type(crate_path = "subxt::ext::scale_encode")]
pub struct Weight {
    #[codec(compact)]
    /// The weight of computational time used based on some reference hardware.
    ref_time: u64,
    #[codec(compact)]
    /// The weight of storage space used by proof of validity.
    proof_size: u64,
}

impl From<sp_weights::Weight> for Weight {
    fn from(weight: sp_weights::Weight) -> Self {
        Self {
            ref_time: weight.ref_time(),
            proof_size: weight.proof_size(),
        }
    }
}

impl From<Weight> for sp_weights::Weight {
    fn from(weight: Weight) -> Self {
        sp_weights::Weight::from_parts(weight.ref_time, weight.proof_size)
    }
}

/// A struct that encodes RPC parameters required to instantiate a new smart contract.
#[derive(Serialize, Encode)]
#[serde(rename_all = "camelCase")]
struct InstantiateRequest {
    origin: AccountId,
    value: Balance,
    gas_limit: Option<Weight>,
    storage_deposit_limit: Option<Balance>,
    code: Code,
    data: Vec<u8>,
    salt: Vec<u8>,
}

/// Reference to an existing code hash or a new Wasm module.
#[derive(Serialize, Encode)]
#[serde(rename_all = "camelCase")]
enum Code {
    /// A Wasm module as raw bytes.
    Upload(Vec<u8>),
    #[allow(unused)]
    /// The code hash of an on-chain Wasm blob.
    Existing(H256),
}

/// A struct that encodes RPC parameters required for a call to a smart contract.
///
/// Copied from [`pallet-contracts-rpc`].
#[derive(Serialize, Encode)]
#[serde(rename_all = "camelCase")]
struct RpcCallRequest {
    origin: AccountId,
    dest: AccountId,
    value: Balance,
    gas_limit: Option<Weight>,
    storage_deposit_limit: Option<Balance>,
    input_data: Vec<u8>,
}
