use super::xts::{
    self,
    api::{
        self,
        ethereum::events::Executed,
        runtime_types::evm_core::error::{ExitReason, ExitSucceed},
    },
    MoonbeamApi
};
use sp_core::H160;
use color_eyre::eyre;
use futures::StreamExt;
use impl_serde::serialize::from_hex;
use secp256k1::SecretKey;
use web3::{
    ethabi::Token,
    signing::{SecretKeyRef, Key},
};

pub struct MoonbeamRunner {
    api: MoonbeamApi,
    signer: SecretKey,
}

impl MoonbeamRunner {
    pub fn new(signer: SecretKey, api: MoonbeamApi) -> Self {
        Self { signer, api }
    }

    pub async fn prepare_contract(&mut self, name: &str, instance_count: u32, ctor_params: &[Token]) -> color_eyre::Result<()> {
        print!("Preparing {name}...");

        let root = std::env::var("CARGO_MANIFEST_DIR")?;
        let bin_path = format!("{root}/contracts/solidity/{name}.bin");
        let metadata_path = format!("{root}/contracts/solidity/{name}_meta.json");
        let code = from_hex(&std::fs::read_to_string(bin_path)?)?;
        let metadata_reader = std::fs::File::open(metadata_path)?;
        let json: serde_json::Map<String, serde_json::Value> =
            serde_json::from_reader(metadata_reader)?;
        let abi = json["output"]["abi"].clone();
        let contract: web3::ethabi::Contract = serde_json::from_value(abi)?;
        let constructor = contract
            .constructor()
            .ok_or_else(|| eyre::eyre!("No constructor for contract found"))?;

        println!("{}KiB", code.len() / 1024);

        let data = constructor.encode_input(code.into(), ctor_params)?;

        let contract_accounts = self
            .exec_deploy(&data, instance_count)
            .await?;

        println!("Instantiated {} {name} contracts", contract_accounts.len());

        // todo: build a set of calls for each contract

        Ok(())
    }

    async fn exec_deploy(&self, data: &[u8], instance_count: u32) -> color_eyre::Result<Vec<H160>> {
        let mut events = self.api.api().events().subscribe().await?.filter_events::<(
            api::system::events::ExtrinsicFailed,
            Executed,
        )>();

        let mut tx_hashes = Vec::new();
        for i in 0..instance_count {
            let tx_hash = self.api.deploy(data, &self.signer).await?;
            tx_hashes.push(tx_hash);
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
                    if from == Key::address(&SecretKeyRef::from(&self.signer)) {
                        match exit_reason {
                            ExitReason::Succeed(ExitSucceed::Returned) => {
                                addresses.push(contract_address);
                                if addresses.len() == instance_count as usize {
                                    break;
                                }
                            }
                            ExitReason::Error(error) => {
                                return Err(eyre::eyre!("Error executing tx {:?}: {:?}", tx, error))
                            }
                            _ => return Err(eyre::eyre!("tx {:?}: exit_reason {:?}", tx, exit_reason))
                        }
                    }
                }
                _ => unreachable!("Only a single event should be emitted at a time"),
            }
        }
        Ok(addresses)
    }
}
