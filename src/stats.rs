use futures::{Future, Stream, TryStream, TryStreamExt};
use std::pin::Pin;
use std::task::{Context, Poll};

use std::collections::HashSet;
use std::sync::{Arc, Mutex};
pub struct BlockInfo {
    pub stats: blockstats::BlockStats,
    // list of hashes to look for
    pub hashes: Vec<sp_core::H256>,
}

struct StreamNoOpUntil<F>
where
    F: FnMut() -> bool,
{
    predicate: F,
}

/// A stream to keep generating empty elements as long as predicate returns true
/// Think of it as a 'CPU Clock' generating 'cycles' as a driver for further operations built upon this
impl<F> Stream for StreamNoOpUntil<F>
where
    F: FnMut() -> bool + Unpin,
{
    type Item = Result<(), subxt::Error>;

    fn poll_next(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if (self.predicate)() {
            Poll::Ready(Some(Ok(())))
        } else {
            Poll::Ready(None)
        }
    }
}

/// Subscribes to block stats, printing the output to the console. Completes once *all* hashes in
/// `remaining_hashes` have been received.
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
    let stream = StreamNoOpUntil {
        predicate: move || {
            let remaining_hashes = remaining_hashes.lock().unwrap();
            !remaining_hashes.is_empty()
        },
    };

    stream
        .map_err(|e| color_eyre::eyre::eyre!("Block stats subscription error: {e:?}"))
        .and_then(move |_| {
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
