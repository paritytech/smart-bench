use futures::{SinkExt, StreamExt};
use sp_runtime::traits::{BlakeTwo256, Hash as _};
use subxt::DefaultConfig;

type Hash = sp_core::H256;

pub struct BlocksSubscription {
    receiver: futures::channel::mpsc::UnboundedReceiver<BlockExtrinsics>,
}

impl BlocksSubscription {
    pub fn wait_for_txs(self, tx_hashes: &[Hash]) -> impl futures::Stream<Item = BlockExtrinsics> {
        let mut remaining_hashes: std::collections::HashSet<Hash> =
            tx_hashes.iter().cloned().collect();

        self.receiver.take_while(move |block_xts| {
            let some_remaining_txs = !remaining_hashes.is_empty();
            for xt in &block_xts.extrinsics {
                remaining_hashes.remove(xt);
            }
            futures::future::ready(some_remaining_txs)
        })
    }
}

impl BlocksSubscription {
    pub async fn new(url: &str) -> color_eyre::Result<Self> {
        let client: subxt::Client<DefaultConfig> =
            subxt::ClientBuilder::new().set_url(url).build().await?;
        let mut blocks_sub = client.rpc().subscribe_blocks().await?;
        let (mut sender, receiver) = futures::channel::mpsc::unbounded();

        tokio::task::spawn(async move {
            while let Some(Ok(block_header)) = blocks_sub.next().await {
                if let Ok(Some(block)) = client.rpc().block(Some(block_header.hash())).await {
                    let extrinsics = block
                        .block
                        .extrinsics
                        .iter()
                        .map(BlakeTwo256::hash_of)
                        .collect();
                    let block_extrinsics = BlockExtrinsics {
                        block_number: block_header.number,
                        block_hash: block_header.hash(),
                        extrinsics,
                    };
                    sender.send(block_extrinsics).await.expect("Send failed");
                }
            }
        });

        Ok(Self { receiver })
    }
}

#[derive(Debug)]
pub struct BlockExtrinsics {
    pub block_number: u32,
    pub block_hash: Hash,
    pub extrinsics: Vec<Hash>,
}
