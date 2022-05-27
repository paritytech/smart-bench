use super::transaction::Transaction;
use impl_serde::serialize::to_hex;
use subxt::{ClientBuilder, DefaultConfig, PolkadotExtrinsicParams};
use web3::{
    signing::Key,
    transports::ws,
    types::{Address, H256, U256},
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
    gas_price: U256,
    chain_id: U256,
}

impl MoonbeamApi {
    pub async fn new(url: &str) -> color_eyre::Result<Self> {
        let transport = ws::WebSocket::new(url).await?;
        let client = ClientBuilder::new().set_url(url).build().await?;
        let web3 = Web3::new(transport);
        let gas_price = web3.eth().gas_price().await?;
        let chain_id = web3.eth().chain_id().await?;
        Ok(Self {
            web3,
            api: client.to_runtime_api(),
            gas_price,
            chain_id,
        })
    }

    pub fn api(&self) -> &api::RuntimeApi<DefaultConfig, PolkadotExtrinsicParams<DefaultConfig>> {
        &self.api
    }

    pub async fn fetch_nonce(&self, address: Address) -> color_eyre::Result<U256> {
        self.web3
            .eth()
            .transaction_count(address, None)
            .await
            .map_err(Into::into)
    }

    pub async fn deploy(
        &self,
        data: &[u8],
        signer: impl Key,
        nonce: U256,
    ) -> color_eyre::Result<H256> {
        let tx = Transaction {
            nonce,
            to: None,
            gas: 1_000_000u32.into(),
            gas_price: self.gas_price,
            value: 0u32.into(),
            data: data.into(),
            transaction_type: None,
            access_list: Default::default(),
            max_priority_fee_per_gas: self.gas_price,
        };

        let signed_tx = tx.sign(signer, self.chain_id.as_u64());

        tracing::debug!("data: {}", to_hex(data, false));
        tracing::debug!(
            "signed_tx.raw_transaction: {}",
            to_hex(&signed_tx.raw_transaction.0, false)
        );
        tracing::debug!("signed_tx.message_hash: {:?}", signed_tx.message_hash);
        tracing::debug!(
            "signed_tx.transaction_hash: {:?}",
            signed_tx.transaction_hash
        );

        let hash = self
            .web3
            .eth()
            .send_raw_transaction(signed_tx.raw_transaction)
            .await?;

        Ok(hash)
    }
}
