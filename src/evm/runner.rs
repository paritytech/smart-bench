use std::collections::HashSet;

use super::xts::{
    api::{
        self,
        ethereum::events::Executed,
        runtime_types::evm_core::error::{ExitReason, ExitSucceed},
    },
    MoonbeamApi,
};
use crate::BlockInfo;
use color_eyre::{eyre, Section as _};
use futures::{StreamExt, TryStream};
use impl_serde::serialize::from_hex;
use secp256k1::SecretKey;
use subxt::{OnlineClient, PolkadotConfig as DefaultConfig};
use web3::{
    ethabi::Token,
    signing::{Key, SecretKeyRef},
    types::{Address, U256},
};

pub struct MoonbeamRunner {
    url: String,
    pub api: MoonbeamApi,
    signer: SecretKey,
    address: Address,
    calls: Vec<(String, Vec<Call>)>,
}

impl MoonbeamRunner {
    pub fn new(url: String, signer: SecretKey, api: MoonbeamApi) -> Self {
        let address = Key::address(&SecretKeyRef::from(&signer));
        Self {
            url,
            signer,
            api,
            address,
            calls: Vec::new(),
        }
    }

    pub async fn prepare_contract<F>(
        &mut self,
        name: &str,
        instance_count: u32,
        ctor_params: &[Token],
        call_name: &str,
        mut create_call_params: F,
    ) -> color_eyre::Result<()>
    where
        F: FnMut() -> Vec<Token>,
    {
        print!("Preparing {name}...");

        let root = std::env::var("CARGO_MANIFEST_DIR")?;
        let metadata_path =
            format!("{root}/contracts/solidity/evm/contracts/{name}.sol/{name}.json");

        let metadata_reader = std::fs::File::open(metadata_path)?;
        let json: serde_json::Map<String, serde_json::Value> =
            serde_json::from_reader(metadata_reader)?;
        let bytecode = json["bytecode"]
            .as_str()
            .ok_or_else(|| eyre::eyre!("Bytecode should be a string"))?;
        let code = from_hex(bytecode).note("Error decoding hex bytecode")?;
        let abi = json["abi"].clone();
        let contract: web3::ethabi::Contract = serde_json::from_value(abi)?;
        let constructor = contract
            .constructor()
            .ok_or_else(|| eyre::eyre!("No constructor for contract found"))?;

        println!("{}KiB", code.len() / 1024);

        let data = constructor.encode_input(code.into(), ctor_params)?;

        let contract_accounts = self.exec_deploy(&data, instance_count).await?;

        println!("Instantiated {} {name} contracts", contract_accounts.len());

        let call = contract
            .function(call_name)
            .with_note(|| format!("Call '{call_name}' not found for {name}"))?;

        let mut calls = Vec::new();
        for contract in contract_accounts {
            let call_params = create_call_params();
            let data = call
                .encode_input(&call_params)
                .note("Error encoding contract call input")?;
            let gas_limit = self
                .api
                .estimate_gas(self.address, Some(contract), &data)
                .await
                .note("Error estimating gas")?;
            calls.push(Call {
                name: name.to_string(),
                contract,
                data,
                gas_limit,
            })
        }
        self.calls.push((name.to_string(), calls));

        Ok(())
    }

    async fn exec_deploy(
        &self,
        data: &[u8],
        instance_count: u32,
    ) -> color_eyre::Result<Vec<Address>> {
        let mut nonce = self.api.fetch_nonce(self.address).await?;
        let mut block_sub = self.api.client().blocks().subscribe_best().await?;

        let gas = self
            .api
            .estimate_gas(self.address, None, &data)
            .await
            .note("Error estimating gas")?;

        let gas_price = self.api.get_gas_price().await.note("Error getting gas")?;

        let mut tx_hashes = HashSet::new();
        for _ in 0..instance_count {
            let tx_hash = self
                .api
                .deploy(data, &self.signer, nonce, gas, gas_price)
                .await?;
            tx_hashes.insert(tx_hash);
            nonce += 1.into();
        }

        let mut addresses = Vec::new();

        while let Some(Ok(block)) = block_sub.next().await {
            let events = block.events().await?;
            for event in events.iter() {
                let event = event?;
                if let Some(Executed {
                    from,
                    to,
                    transaction_hash,
                    exit_reason,
                }) = event.as_event::<Executed>()?
                {
                    tracing::debug!("still expecting {:?}, now got {:?}", tx_hashes, transaction_hash);
                    // When deploying multiple contracts (--instance-count >1), it may happen that here we are processing
                    // a block related to previous contract's deployment
                    //
                    // make sure we are examining transactions related to current deployment and skip otherwise
                    if !tx_hashes.remove(&transaction_hash) {
                        continue;
                    };

                    if from.as_ref() == Key::address(&SecretKeyRef::from(&self.signer)).as_ref() {
                        match exit_reason {
                            ExitReason::Succeed(ExitSucceed::Returned) => {
                                tracing::debug!("Deployed contract {}", to.0);
                                addresses.push(Address::from_slice(to.as_ref()));
                                if addresses.len() == instance_count as usize {
                                    return Ok(addresses);
                                }
                            }
                            ExitReason::Error(error) => {
                                return Err(eyre::eyre!(
                                    "Error executing tx {:?}: {:?}",
                                    transaction_hash,
                                    error
                                ))
                            }
                            _ => {
                                return Err(eyre::eyre!(
                                    "tx {:?}: exit_reason {:?}",
                                    transaction_hash,
                                    exit_reason
                                ))
                            }
                        }
                    }
                } else if event
                    .as_event::<api::system::events::ExtrinsicFailed>()?
                    .is_some()
                {
                    let metadata = self.api.client.metadata();
                    let dispatch_error =
                        subxt::error::DispatchError::decode_from(event.field_bytes(), metadata);
                    return Err(eyre::eyre!("Deploy Extrinsic Failed: {:?}", dispatch_error));
                }
            }
        }
        Err(eyre::eyre!(
            "Expected {} Executed Success events, received {}",
            instance_count,
            addresses.len()
        ))
    }

    /// eth_sendRawTransaction rpc response contains ethereum transaction
    /// hashes instead of extrinsics hashes
    ///
    /// for given block, ethereum transaction hash can be retrieved
    /// from events of type ethereum.Executed
    async fn get_eth_hashes_from_events_in_block(
        client: OnlineClient<DefaultConfig>,
        block_hash: sp_core::H256,
    ) -> color_eyre::Result<Vec<sp_core::H256>> {
        let events = client.events().at(block_hash).await?;
        let mut tx_hashes = Vec::new();
        for event in events.iter() {
            let event = event?;
            if let Some(Executed {
                transaction_hash, ..
            }) = event.as_event::<Executed>()?
            {
                tx_hashes.push(transaction_hash);
            }
        }
        Ok(tx_hashes)
    }

    /// Call each contract instance `call_count` times. Wait for all txs to be included in a block
    /// before returning.
    pub async fn run(
        &mut self,
        call_count: u32,
    ) -> color_eyre::Result<impl TryStream<Ok = BlockInfo, Error = color_eyre::Report> + '_> {
        let block_stats = blockstats::subscribe_stats(&self.url).await?;

        let mut tx_hashes = Vec::new();
        let max_instance_count = self
            .calls
            .iter()
            .map(|(_, calls)| calls.len())
            .max()
            .ok_or_else(|| eyre::eyre!("No prepared contracts for benchmarking."))?;
        let mut nonce = self.api.fetch_nonce(self.address).await?;
        let gas_price = self.api.get_gas_price().await.note("Error getting gas")?;

        for _ in 0..call_count {
            for i in 0..max_instance_count {
                for (_name, contract_calls) in &self.calls {
                    if let Some(contract_call) = contract_calls.get(i as usize) {
                        tracing::debug!(
                            "Calling {}, address {}, gas_limit {}",
                            contract_call.name,
                            contract_call.contract,
                            contract_call.gas_limit
                        );
                        let tx_hash = self
                            .api
                            .call(
                                contract_call.contract,
                                &contract_call.data,
                                &self.signer,
                                nonce,
                                contract_call.gas_limit,
                                gas_price,
                            )
                            .await?;
                        nonce += 1.into();
                        tx_hashes.push(tx_hash)
                    }
                }
            }
        }

        println!("Submitted {} total contract calls", tx_hashes.len());

        let remaining_hashes: std::collections::HashSet<sp_core::H256> = tx_hashes
            .iter()
            .map(|hash| sp_core::H256::from_slice(hash.as_ref()))
            .collect();

        let wait_for_txs = crate::collect_block_stats(block_stats, remaining_hashes, |hash| {
            let client = self.api.client.clone();
            Self::get_eth_hashes_from_events_in_block(client, hash)
        });

        Ok(wait_for_txs)
    }
}

struct Call {
    name: String,
    contract: Address,
    data: Vec<u8>,
    gas_limit: U256,
}
