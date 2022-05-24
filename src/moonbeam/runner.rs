use super::xts::{self, api, MoonbeamApi};
use color_eyre::eyre;
use futures::StreamExt;

pub struct MoonbeamRunner {
    api: MoonbeamApi,
}

impl MoonbeamRunner {
    pub fn new(api: MoonbeamApi) -> Self {
        Self { api }
    }

    pub async fn exec_deploy(&self, data: Vec<u8>) -> color_eyre::Result<()> {
        let mut events = self.api.api().events().subscribe().await?.filter_events::<(
            api::system::events::ExtrinsicFailed,
            api::ethereum::events::Executed,
        )>();

        self.api.deploy(data, &xts::alice()).await?;

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
                (None, Some(executed)) => {
                    // todo: add account id to vec
                    println!("EXECUTED! {:?}", executed);
                    return Ok(());
                }
                _ => unreachable!("Only a single event should be emitted at a time"),
            }
        }
        Ok(())
    }
}
