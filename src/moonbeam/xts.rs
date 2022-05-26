use super::transaction::Transaction;
use impl_serde::serialize::to_hex;
use subxt::{ClientBuilder, DefaultConfig, PolkadotExtrinsicParams};
use web3::{
    signing::Key,
    transports::ws,
    types::{H256},
    Web3,
};

#[subxt::subxt(runtime_metadata_path = "metadata/moonbeam.scale")]
pub mod api {
    #[subxt(substitute_type = "primitive_types::H160")]
    use sp_core::H160;
}

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

        tracing::info!("nonce {}, gas_price {}, chain_id {}", nonce, gas_price, chain_id);

        let tx = Transaction {
            nonce,
            to: None,
            gas: 1_000_000u32.into(),
            gas_price,
            value: 0u32.into(),
            data: data.into(),
            transaction_type: None,
            access_list: Default::default(),
            max_priority_fee_per_gas: gas_price,
        };

        let signed_tx = tx.sign(signer, chain_id.as_u64());

        tracing::debug!("data: {}", to_hex(data, false));
        tracing::debug!("signed_tx.raw_transaction: {}", to_hex(&signed_tx.raw_transaction.0, false));
        tracing::debug!("signed_tx.message_hash: {:?}", signed_tx.message_hash);
        tracing::debug!("signed_tx.transaction_hash: {:?}", signed_tx.transaction_hash);

        let hash = self
            .web3
            .eth()
            .send_raw_transaction(signed_tx.raw_transaction)
            .await?;

        Ok(hash)
    }
}
