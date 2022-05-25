use super::transaction::Transaction;
use color_eyre::eyre;
use secp256k1::SecretKey;
use std::str::FromStr;
use serde::Serialize;
use subxt::{ClientBuilder, DefaultConfig, PolkadotExtrinsicParams};
use web3::{
    signing::Key,
    transports::ws,
    types::{Address, Bytes, TransactionParameters, H256, H160, U256, U64},
    Web3,
};

#[subxt::subxt(runtime_metadata_path = "metadata/moonbeam.scale")]
pub mod api {}

pub struct MoonbeamApi {
    web3: Web3<ws::WebSocket>,
    api: api::RuntimeApi<DefaultConfig, PolkadotExtrinsicParams<DefaultConfig>>,
}

impl MoonbeamApi {
    pub async fn new(url: &str) -> color_eyre::Result<Self> {
        let transport = ws::WebSocket::new(url).await?;
        let client = ClientBuilder::new().set_url(url).build().await?;
        Ok(Self {
            web3: Web3::new(transport),
            api: client.to_runtime_api(),
        })
    }

    pub fn api(&self) -> &api::RuntimeApi<DefaultConfig, PolkadotExtrinsicParams<DefaultConfig>> {
        &self.api
    }

    pub async fn deploy(&self, data: &[u8], signer: impl Key) -> color_eyre::Result<H256> {
        let nonce = self
            .web3
            .eth()
            .transaction_count(signer.address(), None)
            .await?;
        let gas_price = self.web3.eth().gas_price().await?;
        let chain_id = self.web3.eth().chain_id().await?;

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

        let hash = self
            .web3
            .eth()
            .send_raw_transaction(signed_tx.raw_transaction)
            .await?;

        Ok(hash)
    }

    pub async fn deploy2(&self, json: &serde_json::Value, code: &str, signer: impl Key) -> color_eyre::Result<H160> {
        let json = serde_json::to_vec(json)?;
        let contract = web3::contract::Contract::deploy(self.web3.eth(), &json)?
            .confirmations(1)
            .options(web3::contract::Options::with(|opt| {
                // opt.value = Some(5.into());
                // opt.gas_price = Some(5.into());
                opt.gas = Some(3_000_000.into());
            }))
            .sign_with_key_and_execute(
                code,
                (1u32, ),
                signer,
                None,
            )
            .await?;

        let contract_address = contract.address();
        Ok(contract_address)
    }
}

pub fn alice() -> SecretKey {
    SecretKey::from_str("5fb92d6e98884f76de468fa3f6278f8807c48bebc13595d45af5bdc4da702133").unwrap()
}

pub fn bob() -> SecretKey {
    SecretKey::from_str("8075991ce870b93a8870eca0c0f91913d12f47948ca0fd25b49c6fa7cdbeee8b").unwrap()
}
