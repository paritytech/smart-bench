use futures::{future, stream::poll_fn, Future, TryStream, TryStreamExt};
use std::task::Poll;

use std::collections::HashSet;
use std::sync::{Arc, Mutex};
pub struct BlockInfo {
    pub stats: blockstats::BlockStats,
    // list of hashes to look for
    pub hashes: Vec<sp_core::H256>,
}

/// Subscribes to block stats. Completes once *all* hashes in `remaining_hashes` have been received.
pub fn collect_block_stats<F, Fut>(
    block_stats: impl TryStream<Ok = blockstats::BlockStats, Error = subxt::Error> + Unpin,
    remaining_hashes: HashSet<sp_core::H256>,
    get_hashes_in_block: F,
) -> impl TryStream<Ok = BlockInfo, Error = color_eyre::Report>
where
    F: Fn(sp_core::H256) -> Fut + Copy,
    Fut: Future<Output = color_eyre::Result<Vec<sp_core::H256>>>,
{
    let block_stats_arc = Arc::new(Mutex::new(block_stats));
    let remaining_hashes_arc = Arc::new(Mutex::new(remaining_hashes));

    let remaining_hashes = remaining_hashes_arc.clone();
    let stream = poll_fn(move |_| -> Poll<Option<Result<(), color_eyre::Report>>> {
        let remaining_hashes = remaining_hashes.lock().unwrap();

        if !remaining_hashes.is_empty() {
            Poll::Ready(Some(Ok(())))
        } else {
            Poll::Ready(None)
        }
    });

    stream.and_then(move |_| {
        let remaining_hashes = remaining_hashes_arc.clone();
        let block_stats = block_stats_arc.clone();
        async move {
            let stats = block_stats.lock().unwrap().try_next().await?.unwrap();
            tracing::debug!("{stats:?}");
            let hashes = get_hashes_in_block(stats.hash).await?;
            let mut remaining_hashes = remaining_hashes.lock().unwrap();
            for xt in &hashes {
                remaining_hashes.remove(xt);
            }
            Ok(BlockInfo { hashes, stats })
        }
    })
}

///  Print the block info stats to the console
pub async fn print_block_info(
    block_info: impl TryStream<Ok = BlockInfo, Error = color_eyre::Report>,
) -> color_eyre::Result<()> {
    let mut total_extrinsics = 0u64;
    let mut total_blocks = 0u64;
    println!();
    block_info
        .try_for_each(|block| {
            println!("{}", block.stats);
            let total_extrinsics_result = total_extrinsics
                .checked_add(block.stats.num_extrinsics)
                .ok_or_else(|| {
                    color_eyre::Report::msg("Overflow occurred when calculating total extrinsics")
                });
            let result = total_extrinsics_result.and_then(|extrinsics| {
                total_extrinsics = extrinsics;
                total_blocks = total_blocks.checked_add(1).ok_or_else(|| {
                    color_eyre::Report::msg("Overflow occurred when calculating total blocks")
                })?;
                Ok(())
            });
            future::ready(result)
        })
        .await?;
    println!("\nSummary:");
    println!("TPS - Transaction execution time per second, assuming a 0.5-second execution time per block");
    println!(
        "TPS: {}",
        total_extrinsics as f64 / (total_blocks as f64 * 0.5)
    );
    Ok(())
}
