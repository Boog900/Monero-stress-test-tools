use clap::Parser;
use monero_serai::rpc::HttpRpc;

const NUMB_TASKS: usize = 5;

#[derive(Parser)]
struct Args {
    /// The node to target the transactions at.
    #[arg(long)]
    target_node: String,
    /// The node to get the transactions from.
    #[arg(long)]
    data_node: String,
    /// The height to start pulling txs from blocks.
    #[arg(long)]
    start_height: usize,
}

// 3139920

#[tokio::main]
async fn main() {
    let args = Args::parse();

    let data_rpc =
        HttpRpc::with_custom_timeout(args.data_node, std::time::Duration::from_secs(600))
            .await
            .unwrap();
    let target_rpc = HttpRpc::new(args.target_node).await.unwrap();

    let height = args.start_height;

    for i in 0..NUMB_TASKS {
        let data_rpc = data_rpc.clone();
        let target_rpc = target_rpc.clone();
        tokio::spawn(async move {
            let mut height = height + i;

            loop {
                let block = data_rpc.get_block_by_number(height).await.unwrap();

                for tx in block.txs {
                    let tx = data_rpc.get_transaction(tx).await.unwrap();

                    let _ = target_rpc.publish_transaction(&tx).await;
                }

                println!("Sent txs from block: {height}");

                height += NUMB_TASKS;
            }
        });
    }

    futures::future::pending().await
}
