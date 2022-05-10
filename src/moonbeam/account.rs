use codec::{Decode, Encode};
use sha3::{Digest, Keccak256};
use sp_core::{ecdsa, H160, H256};

#[derive(
    Eq, PartialEq, Copy, Clone, Encode, Decode, Default, PartialOrd, Ord,
)]
pub struct AccountId20(pub [u8; 20]);

impl_serde::impl_fixed_hash_serde!(AccountId20, 20);

impl core::fmt::Debug for AccountId20 {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:?}", sp_core::H160(self.0))
    }
}

#[derive(Eq, PartialEq, Clone, Encode, Decode, sp_core::RuntimeDebug)]
pub struct EthereumSignature(ecdsa::Signature);

impl From<ecdsa::Signature> for EthereumSignature {
    fn from(x: ecdsa::Signature) -> Self {
        EthereumSignature(x)
    }
}

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

impl From<ecdsa::Public> for EthereumSigner {
    fn from(x: ecdsa::Public) -> Self {
        let decompressed = libsecp256k1::PublicKey::parse_slice(
            &x.0,
            Some(libsecp256k1::PublicKeyFormat::Compressed),
        )
        .expect("Wrong compressed public key provided")
        .serialize();
        let mut m = [0u8; 64];
        m.copy_from_slice(&decompressed[1..65]);
        let account = H160::from(H256::from_slice(Keccak256::digest(&m).as_slice()));
        EthereumSigner(account.into())
    }
}
