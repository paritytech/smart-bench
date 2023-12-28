use super::transaction::Transaction;
use impl_serde::serialize::to_hex;
use subxt::{OnlineClient, PolkadotConfig as DefaultConfig};
use web3::{
    signing::Key,
    transports::ws,
    types::{Address, CallRequest, H256, U256},
    Web3,
};

#[subxt::subxt(
    runtime_metadata_path = "metadata/moonbeam.scale",
    substitute_type(
        path = "primitive_types::H160",
        with = "::subxt::utils::Static<::sp_core::H160>"
    )
)]
pub mod api {}

pub struct MoonbeamApi {
    web3: Web3<ws::WebSocket>,
    pub client: OnlineClient<DefaultConfig>,
    chain_id: U256,
}

impl MoonbeamApi {
    pub async fn new(url: &str) -> color_eyre::Result<Self> {
        let transport = ws::WebSocket::new(url).await?;
        let client = OnlineClient::from_url(url).await?;
        let web3 = Web3::new(transport);
        let chain_id = web3.eth().chain_id().await?;
        Ok(Self {
            web3,
            client,
            chain_id,
        })
    }

    pub fn client(&self) -> &OnlineClient<DefaultConfig> {
        &self.client
    }

    pub async fn fetch_nonce(&self, address: Address) -> color_eyre::Result<U256> {
        self.web3
            .eth()
            .transaction_count(address, None)
            .await
            .map_err(Into::into)
    }

    /// Estimate gas price for transaction
    ///
    /// In Moonbeam, the gas price is subject to change from block to block.
    /// In smart-bench, contract deployment transactions are sent at once and spread over many blocks.
    /// The same is for contract call extrinsic.
    /// The gas price estimated at the beginning may not be valid for transactions processed in subsequent blocks.
    /// Considering that the base fee can increase by a maximum of 12.5% per block if the target block size is exceeded,
    /// the returned gas price is enlarged by 12.5%.
    pub async fn get_gas_price(&self) -> color_eyre::Result<U256> {
        let gas_price = self.web3.eth().gas_price().await?.to_f64_lossy();
        Ok(U256::from_f64_lossy(gas_price * 1.125))
    }

    pub async fn estimate_gas(
        &self,
        from: Address,
        contract: Option<Address>,
        data: &[u8],
    ) -> color_eyre::Result<U256> {
        let call_request = CallRequest {
            from: Some(from),
            to: contract,
            gas: None,
            gas_price: None,
            value: None,
            data: Some(data.into()),
            transaction_type: None,
            access_list: None,
            max_fee_per_gas: None,
            max_priority_fee_per_gas: None,
        };
        self.web3
            .eth()
            .estimate_gas(call_request, None)
            .await
            .map_err(Into::into)
    }

    pub async fn deploy(
        &self,
        data: &[u8],
        signer: impl Key,
        nonce: U256,
        gas: U256,
        gas_price: U256,
    ) -> color_eyre::Result<H256> {
        self.sign_and_submit_tx(data, signer, nonce, None, gas, gas_price)
            .await
    }

    pub async fn call(
        &self,
        contract: Address,
        data: &[u8],
        signer: impl Key,
        nonce: U256,
        gas: U256,
        gas_price: U256,
    ) -> color_eyre::Result<H256> {
        self.sign_and_submit_tx(data, signer, nonce, Some(contract), gas, gas_price)
            .await
    }

    pub async fn sign_and_submit_tx(
        &self,
        data: &[u8],
        signer: impl Key,
        nonce: U256,
        to: Option<Address>,
        gas: U256,
        gas_price: U256,
    ) -> color_eyre::Result<H256> {
        let tx = Transaction {
            nonce,
            to,
            gas,
            gas_price,
            value: 0u32.into(),
            data: data.into(),
            transaction_type: None,
            access_list: Default::default(),
            max_priority_fee_per_gas: gas_price,
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
