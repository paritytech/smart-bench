use codec::{Decode, Encode};
use color_eyre::eyre;
use sha3::{Digest, Keccak256};
use sp_core::{ecdsa, serde, H160, H256, U256};
use subxt::{DefaultConfig, DefaultExtra, PairSigner};

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

#[derive(
    Eq, PartialEq, Copy, Clone, Encode, Decode, Default, PartialOrd, Ord, serde::Serialize,
)]
pub struct AccountId20(pub [u8; 20]);

impl core::fmt::Debug for AccountId20 {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:?}", sp_core::H160(self.0))
    }
}

#[derive(Eq, PartialEq, Clone, Encode, Decode, sp_core::RuntimeDebug)]
pub struct EthereumSignature(ecdsa::Signature);

impl sp_runtime::traits::Verify for EthereumSignature {
    type Signer = EthereumSigner;
    fn verify<L: sp_runtime::traits::Lazy<[u8]>>(&self, mut msg: L, signer: &AccountId20) -> bool {
        let mut m = [0u8; 32];
        m.copy_from_slice(Keccak256::digest(msg.get()).as_slice());
        match sp_io::crypto::secp256k1_ecdsa_recover(self.0.as_ref(), &m) {
            Ok(pubkey) => {
                AccountId20(H160::from(H256::from_slice(Keccak256::digest(&pubkey).as_slice())).0)
                    == *signer
            }
            Err(sp_io::EcdsaVerifyError::BadRS) => false,
            Err(sp_io::EcdsaVerifyError::BadV) => false,
            Err(sp_io::EcdsaVerifyError::BadSignature) => false,
        }
    }
}

/// Public key for an Ethereum / Moonbeam compatible account
#[derive(Eq, PartialEq, Ord, PartialOrd, Clone, Encode, Decode, sp_core::RuntimeDebug)]
pub struct EthereumSigner([u8; 20]);

impl sp_runtime::traits::IdentifyAccount for EthereumSigner {
    type AccountId = AccountId20;
    fn into_account(self) -> AccountId20 {
        AccountId20(self.0)
    }
}

impl From<[u8; 20]> for EthereumSigner {
    fn from(x: [u8; 20]) -> Self {
        EthereumSigner(x)
    }
}

#[subxt::subxt(runtime_metadata_path = "metadata/moonbeam.scale")]
pub mod api {
    #[subxt(substitute_type = "primitive_types::H160")]
    use sp_core::H160;
    #[subxt(substitute_type = "primitive_types::U256")]
    use sp_core::U256;
}

pub type Signer = PairSigner<DefaultConfig, DefaultExtra<DefaultConfig>, ecdsa::Pair>;

pub struct MoonbeamApi {
    api: api::RuntimeApi<DefaultConfig, DefaultExtra<DefaultConfig>>,
}

impl MoonbeamApi {
    pub fn new(client: subxt::Client<DefaultConfig>) -> Self {
        let api =
            client.to_runtime_api::<api::RuntimeApi<DefaultConfig, DefaultExtra<DefaultConfig>>>();
        Self { api }
    }

    pub async fn create(
        &self,
        from: H160,
        data: Vec<u8>,
        value: U256,
        gas_limit: u64,
        nonce: Option<U256>,
        signer: &Signer,
    ) -> color_eyre::Result<AccountId20> {
        let max_fee_per_gas = U256::max_value();
        let max_priority_fee_per_gas = None;
        let access_list = Vec::new();
        let result = self
            .api
            .tx()
            .evm()
            .create(
                from,
                data,
                value,
                gas_limit,
                max_fee_per_gas,
                nonce,
                max_priority_fee_per_gas,
                access_list,
            )
            .sign_and_submit_then_watch(signer)
            .await?
            .wait_for_in_block()
            .await?
            .wait_for_success()
            .await?;

        let created = result
            .find_first::<api::evm::events::Created>()?
            .ok_or_else(|| eyre::eyre!("Failed to find Instantiated event"))?;

        Ok(AccountId20(created.0 .0))
    }
}
