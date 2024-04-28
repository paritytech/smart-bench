use futures::{future, stream::poll_fn, Future, TryStream, TryStreamExt};
use std::task::Poll;

use std::collections::HashSet;
use std::sync::{Arc, Mutex};
pub struct BlockInfo {
    // block time stamp
    pub time_stamp: u64,
    pub stats: blockstats::BlockStats,
    // list of hashes to look for
    pub contract_call_hashes: Vec<sp_core::H256>,
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
                contract_call_hashes: hashes,
                stats,
            })
        }
    })
}

/// This function prints statistics to the standard output.

/// The TPS calculation is based on the following assumptions about smart-bench:
/// - smart-bench instantiates smart contracts on the chain and waits for the completion of these transactions.
/// - Starting from some future block (after creation), smart-bench uploads transactions related to contract calls to the node.
/// - Sending contract call transactions to the node is continuous once started and is not mixed with any other type of transactions.
/// - Smart-bench finishes benchmarking at the block that contains the last contract call from the set.

/// TPS calculation is exclusively concerned with contract calls, disregarding any system or contract-creating transactions.

/// TPS calculation excludes the last block of the benchmark, as its full utilization is not guaranteed. In other words, only blocks in the middle will consist entirely of contract calls.
pub async fn print_block_info(
    block_info: impl TryStream<Ok = BlockInfo, Error = color_eyre::Report>,
) -> color_eyre::Result<()> {
    let mut call_extrinsics_per_block: Vec<u64> = Vec::new();
    let mut call_block_expected = false;
    let mut time_stamp = None;
    let mut time_diff = None;
    println!();
    block_info
        .try_for_each(|block| {
            println!("{}", block.stats);
            let contract_calls_count = block.contract_call_hashes.len() as u64;
            // Skip blocks at the beggining until we see first call related transaction
            // Once first call is seen, we expect all further blocks to contain calls until all calls are covered
            if !call_block_expected && contract_calls_count > 0 {
                call_block_expected = true;
            }
            if call_block_expected {
                call_extrinsics_per_block.push(contract_calls_count);
            }

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

    // Skip the last block as it's not stressed to its full capabilities, 
    // since there is a very low chance of hitting that exact amount of transactions 
    // (it will contain as many transactions as there are left to execute).
    let call_extrinsics_per_block =
        &call_extrinsics_per_block[0..call_extrinsics_per_block.len() - 1];

    let tps_blocks = call_extrinsics_per_block.len();
    let tps_total_extrinsics = call_extrinsics_per_block.iter().sum::<u64>();
    println!("\nSummary:");
    println!("Total Blocks: {tps_blocks}");
    println!("Total Extrinsics: {tps_total_extrinsics}");
    let diff = time_diff.unwrap_or_else(|| {
        // default block build time
        let default = 12.0;
        println!("Warning: Could not calculate block build time, assuming {default}");
        default
    });
    println!("Block Build Time: {diff}");
    if tps_blocks > 0 {
        println!("sTPS - Standard Transaction Per Second");
        println!(
            "sTPS: {:.2}",
            tps_total_extrinsics as f64 / (tps_blocks as f64 * diff)
        );
    } else {
        println!("sTPS - Error - not enough data to calculate sTPS, consider increasing --call-count value")
    }
    Ok(())
}
