use futures::{future, stream::poll_fn, Future, TryStream, TryStreamExt};
use std::task::Poll;

use std::collections::HashSet;
use std::sync::{Arc, Mutex};
pub struct BlockInfo {
    // block time stamp
    pub time_stamp: u64,
    pub stats: blockstats::BlockStats,
    // list of hashes to look for
    pub hashes: Vec<sp_core::H256>,
}

/// Subscribes to block stats. Completes once *all* hashes in `remaining_hashes` have been received.
pub fn collect_block_stats<F, Fut>(
    block_stats: impl TryStream<Ok = blockstats::BlockStats, Error = subxt::Error> + Unpin,
    remaining_hashes: HashSet<sp_core::H256>,
    get_block_details: F,
) -> impl TryStream<Ok = BlockInfo, Error = color_eyre::Report>
where
    Fut: Future<Output = color_eyre::Result<(u64, Vec<sp_core::H256>)>>,
    F: Fn(sp_core::H256) -> Fut + Copy,
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
            let (time_stamp, hashes) = get_block_details(stats.hash).await?;
            let mut remaining_hashes = remaining_hashes.lock().unwrap();
            for xt in &hashes {
                remaining_hashes.remove(xt);
            }
            Ok(BlockInfo {
                time_stamp,
                hashes,
                stats,
            })
        }
    })
}

///  Print the block info stats to the console
pub async fn print_block_info(
    block_info: impl TryStream<Ok = BlockInfo, Error = color_eyre::Report>,
) -> color_eyre::Result<()> {
    let mut total_extrinsics = 0u64;
    let mut total_blocks = 0u64;
    let mut time_stamp = None;
    let mut time_diff = None;
    println!();
    block_info
        .try_for_each(|block| {
            println!("{}", block.stats);
            total_extrinsics += block.stats.num_extrinsics;
            total_blocks += 1;
            if time_diff.is_none() {
                if let Some(ts) = time_stamp {
                    time_diff = Some((block.time_stamp - ts) as f64 / 1000.0)
                } else {
                    time_stamp = Some(block.time_stamp)
                }
            }
            future::ready(Ok(()))
        })
        .await?;
    println!("\nSummary:");
    println!("Total Blocks: {total_blocks}");
    println!("Total Extrinsics: {total_extrinsics}");
    let diff = time_diff.unwrap_or_else(|| {
        // default block build time
        let default = 12.0;
        println!("Warning: Could not calculate block build time, assuming {default}");
        default
    });
    println!("Block Build Time: {diff}");
    println!("sTPS - Standard Transaction per Second");
    println!(
        "sTPS: {}",
        total_extrinsics as f64 / (total_blocks as f64 * diff)
    );
    Ok(())
}
