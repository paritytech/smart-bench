use super::xts::{
    self,
    api::{
        self,
        ethereum::events::Executed,
        runtime_types::evm_core::error::{ExitReason, ExitSucceed},
    },
    MoonbeamApi
};
use color_eyre::eyre;
use futures::StreamExt;
use web3::types::H160;

pub struct MoonbeamRunner {
    api: MoonbeamApi,
}

impl MoonbeamRunner {
    pub fn new(api: MoonbeamApi) -> Self {
        Self { api }
    }

    pub async fn exec_deploy(&self, data: &[u8]) -> color_eyre::Result<H160> {
        let mut events = self.api.api().events().subscribe().await?.filter_events::<(
            api::system::events::ExtrinsicFailed,
            Executed,
        )>();

        let tx_hash = self.api.deploy(data, &xts::alice()).await?;

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
                    if tx.as_ref() == tx_hash.as_ref() {
                        return match exit_reason {
                            ExitReason::Succeed(ExitSucceed::Returned) => {
                                Ok(contract_address)
                            }
                            ExitReason::Error(error) => {
                                Err(eyre::eyre!("Error executing tx {:?}: {:?}", tx, error))
                            }
                            _ => Err(eyre::eyre!("tx {:?}: exit_reason {:?}", tx, exit_reason))
                        }
                    }
                }
                _ => unreachable!("Only a single event should be emitted at a time"),
            }
        }
        Err(eyre::eyre!("No triggered events found for attempted contract deployment"))
    }

    pub async fn exec_deploy2(
        &self,
        json: &serde_json::Value,
        code: &str,
    ) -> color_eyre::Result<web3::types::H160> {
        let contract_address = dbg!(self.api.deploy2(json, code, &xts::alice()).await?);
        Ok(contract_address)
    }
}
