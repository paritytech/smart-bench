use color_eyre::eyre;
use web3::{
    Web3,
    signing::Key,
    transports::ws,
    types::{Address, Bytes, TransactionParameters, U64, U256, H256},
};
use secp256k1::SecretKey;
use super::transaction::{
    Transaction,
};
use std::str::FromStr;
use subxt::PolkadotExtrinsicParams;
//
// #[derive(Debug)]
// pub enum MoonbeamConfig {}
//
// impl subxt::Config for MoonbeamConfig {
//     type Index = u32;
//     type BlockNumber = u32;
//     type Hash = H256;
//     type Hashing = sp_runtime::traits::BlakeTwo256;
//     type AccountId = AccountId20;
//     type Address = Self::AccountId;
//     type Header = sp_runtime::generic::Header<Self::BlockNumber, sp_runtime::traits::BlakeTwo256>;
//     type Signature = EthereumSignature;
//     type Extrinsic = sp_runtime::OpaqueExtrinsic;
// }



#[subxt::subxt(runtime_metadata_path = "metadata/moonbeam.scale")]
pub mod api { }

pub struct MoonbeamApi {
    web3: Web3<ws::WebSocket>,
}

impl MoonbeamApi {
    pub fn new(transport: ws::WebSocket) -> Self {
        Self { web3: Web3::new(transport) }
    }

    pub async fn deploy(
        &self,
        data: Vec<u8>,
        signer: impl Key,
    ) -> color_eyre::Result<H256> {

        let nonce = self.web3.eth().transaction_count(signer.address(), None).await?;
        let gas_price = self.web3.eth().gas_price().await?;
        let chain_id = self.web3.eth().chain_id().await?;

        // let max_priority_fee_per_gas = match tx.transaction_type {
        //     Some(tx_type) if tx_type == U64::from(EIP1559_TX_ID) => {
        //         tx.max_priority_fee_per_gas.unwrap_or(gas_price)
        //     }
        //     _ => gas_price,
        // };

        let tx = Transaction {
            nonce,
            to: None,
            gas: 1_000_000u32.into(),
            gas_price: gas_price.into(),
            value: 0u32.into(),
            data: data.into(),
            transaction_type: None,
            access_list: Default::default(),
            max_priority_fee_per_gas: gas_price,
        };

        let signed_tx = tx.sign(signer, chain_id.as_u64());
        let hash = self.web3.eth().send_raw_transaction(signed_tx.raw_transaction).await?;
        Ok(hash)
    }

    // pub async fn call(
    //     &self,
    //     source: H160,
    //     target: H160,
    //     input: Vec<u8>,
    //     value: U256,
    //     gas_limit: u64,
    //     nonce: Option<U256>,
    //     signer: &EthereumPairSigner,
    // ) -> color_eyre::Result<H256> {
    //     let max_fee_per_gas = U256::max_value();
    //     let max_priority_fee_per_gas = None;
    //     let access_list = Vec::new();
    //     let tx_hash = self
    //         .web3
    //         .tx()
    //         .evm()
    //         .call(
    //             source,
    //             target,
    //             input,
    //             value,
    //             gas_limit,
    //             max_fee_per_gas,
    //             nonce,
    //             max_priority_fee_per_gas,
    //             access_list,
    //         )?
    //         .sign_and_submit_default(signer)
    //         .await?;
    //
    //     Ok(tx_hash)
    // }
}

pub fn alice() -> SecretKey {
    SecretKey::from_str("5fb92d6e98884f76de468fa3f6278f8807c48bebc13595d45af5bdc4da702133").unwrap()
}

pub fn bob() -> SecretKey {
    SecretKey::from_str("8075991ce870b93a8870eca0c0f91913d12f47948ca0fd25b49c6fa7cdbeee8b").unwrap()
}
