use futures::{future, Future, TryStream, TryStreamExt};

use std::collections::HashSet;

pub struct BlockInfo {
    pub stats: blockstats::BlockStats,
    // list of hashes to look for
    pub hashes: Vec<sp_core::H256>,
}

/// Subscribes to block stats, printing the output to the console. Completes once *all* hashes in
/// `remaining_hashes` have been received.
pub fn collect_block_stats<F, Fut>(
    block_stats: impl TryStream<Ok = blockstats::BlockStats, Error = subxt::Error>,
    mut remaining_hashes: HashSet<sp_core::H256>,
    get_hashes_in_block: F,
) -> impl TryStream<Ok = BlockInfo, Error = color_eyre::Report>
where
    F: Fn(sp_core::H256) -> Fut + Copy,
    Fut: Future<Output = color_eyre::Result<Vec<sp_core::H256>>>,
{
    block_stats
        .map_err(|e| color_eyre::eyre::eyre!("Block stats subscription error: {e:?}"))
        .and_then(move |stats| {
            tracing::debug!("{stats:?}");
            async move {
                let hashes = get_hashes_in_block(stats.hash).await?;
                Ok(BlockInfo { hashes, stats })
            }
        })
        .try_take_while(move |block_info| {
            if !remaining_hashes.is_empty() {
                for xt in &block_info.hashes {
                    remaining_hashes.remove(xt);
                }
                future::ready(Ok(true))
            } else {
                future::ready(Ok(false))
            }
        })
}
