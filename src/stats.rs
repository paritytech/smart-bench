use futures::{future, TryStream, TryStreamExt};
use sp_runtime::traits::{BlakeTwo256, Hash as _};
use std::collections::HashSet;
use subxt::{Error, OnlineClient, PolkadotConfig as DefaultConfig};

pub struct BlockInfo {
    pub stats: blockstats::BlockStats,
    pub extrinsics: Vec<sp_core::H256>,
}

/// Subscribes to block stats, printing the output to the console. Completes once *all* hashes in
/// `remaining_hashes` have been received.
pub fn collect_block_stats<'a>(
    client: &'a OnlineClient<DefaultConfig>,
    block_stats: impl TryStream<Ok = blockstats::BlockStats, Error = Error> + Unpin + 'a,
    mut remaining_hashes: HashSet<sp_core::H256>,
) -> impl TryStream<Ok = BlockInfo, Error = color_eyre::Report> + Unpin + 'a {
    Box::pin(
        block_stats
            .map_err(|e| color_eyre::eyre::eyre!("Block stats subscription error: {e:?}"))
            .and_then(|stats| {
                tracing::debug!("{stats:?}");
                let client = client.clone();
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
            }),
    )
}
