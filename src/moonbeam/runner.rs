use super::xts::{
    api::{
        self,
        ethereum::events::Executed,
        runtime_types::evm_core::error::{ExitReason, ExitSucceed},
    },
    MoonbeamApi,
};
use color_eyre::{eyre, Section as _};
use futures::{future, StreamExt, TryStream, TryStreamExt};
use impl_serde::serialize::from_hex;
use secp256k1::SecretKey;
use sp_runtime::traits::{BlakeTwo256, Hash as _};
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
    where F: FnMut() -> Vec<Token>,
    {
        print!("Preparing {name}...");

        let root = std::env::var("CARGO_MANIFEST_DIR")?;
        let metadata_path = format!("{root}/contracts/solidity/artifacts/contracts/{name}.sol/{name}.json");

        let metadata_reader = std::fs::File::open(metadata_path)?;
        let json: serde_json::Map<String, serde_json::Value> =
            serde_json::from_reader(metadata_reader)?;
        let bytecode = json["deployedBytecode"].as_str().ok_or_else(|| eyre::eyre!("Bytecode should be a string"))?;
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
            let data = call.encode_input(&call_params)?;
            let gas_limit = self
                .api
                .estimate_gas(self.address, contract, &data)
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
        let mut events = self
            .api
            .api()
            .events()
            .subscribe()
            .await?
            .filter_events::<(api::system::events::ExtrinsicFailed, Executed)>();

        let mut tx_hashes = Vec::new();
        for _ in 0..instance_count {
            let tx_hash = self.api.deploy(data, &self.signer, nonce).await?;
            tx_hashes.push(tx_hash);
            nonce += 1.into();
        }

        let mut addresses = Vec::new();
        while let Some(Ok(info)) = events.next().await {
            match info.event {
                (Some(failed), None) => {
                    let error_data =
                        subxt::HasModuleError::module_error_data(&failed.dispatch_error).ok_or(
                            eyre::eyre!("Failed to find error details for {:?},", failed),
                        )?;
                    let description = {
                        let metadata = self.api.api().client.metadata();
                        let locked_metadata = metadata.read();
                        let details = locked_metadata
                            .error(error_data.pallet_index, error_data.error_index())?;
                        details.description().to_vec()
                    };

                    return Err(eyre::eyre!("Deploy Extrinsic Failed: {:?}", description));
                }
                (None, Some(Executed(from, contract_address, tx, exit_reason))) => {
                    if from.as_ref() == Key::address(&SecretKeyRef::from(&self.signer)).as_ref() {
                        match exit_reason {
                            ExitReason::Succeed(ExitSucceed::Returned) => {
                                addresses.push(Address::from_slice(contract_address.as_ref()));
                                if addresses.len() == instance_count as usize {
                                    break;
                                }
                            }
                            ExitReason::Error(error) => {
                                return Err(eyre::eyre!("Error executing tx {:?}: {:?}", tx, error))
                            }
                            _ => {
                                return Err(eyre::eyre!(
                                    "tx {:?}: exit_reason {:?}",
                                    tx,
                                    exit_reason
                                ))
                            }
                        }
                    }
                }
                _ => unreachable!("Only a single event should be emitted at a time"),
            }
        }
        Ok(addresses)
    }

    /// Call each contract instance `call_count` times. Wait for all txs to be included in a block
    /// before returning.
    pub async fn run(
        &mut self,
        call_count: u32,
    ) -> color_eyre::Result<impl TryStream<Ok = BlockInfo, Error = color_eyre::Report> + '_> {
        let block_stats = povstats::subscribe_stats(&self.url).await?;

        let mut tx_hashes = Vec::new();
        let max_instance_count = self
            .calls
            .iter()
            .map(|(_, calls)| calls.len())
            .max()
            .ok_or_else(|| eyre::eyre!("No prepared contracts for benchmarking."))?;
        let mut nonce = self.api.fetch_nonce(self.address).await?;

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
                            )
                            .await?;
                        nonce += 1.into();
                        tx_hashes.push(tx_hash)
                    }
                }
            }
        }

        println!("Submitted {} total contract calls", tx_hashes.len());

        let mut remaining_hashes: std::collections::HashSet<sp_core::H256> = tx_hashes
            .iter()
            .map(|hash| sp_core::H256::from_slice(hash.as_ref()))
            .collect();

        // todo: this can probably be extracted and duplication with bench runner removed
        let wait_for_txs = block_stats
            .map_err(|e| eyre::eyre!("Block stats subscription error: {e:?}"))
            .and_then(|stats| {
                let client = self.api.api.client.clone();
                async move {
                    let block = client.rpc().block(Some(stats.hash)).await?;
                    let extrinsics = block
                        .unwrap_or_else(|| panic!("block {} not found", stats.hash))
                        .block
                        .extrinsics
                        .iter()
                        .map(BlakeTwo256::hash_of)
                        .collect();
                    Ok(BlockInfo { extrinsics, stats })
                }
            })
            .try_take_while(move |block_info| {
                let some_remaining_txs = !remaining_hashes.is_empty();
                for xt in &block_info.extrinsics {
                    remaining_hashes.remove(xt);
                }
                future::ready(Ok(some_remaining_txs))
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

pub struct BlockInfo {
    pub stats: povstats::BlockStats,
    pub extrinsics: Vec<sp_core::H256>,
}
