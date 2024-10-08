use std::{sync::mpsc, time::Duration};
use tokio::time::sleep;

use harness::{
    constant::THREE_NODE_EXAMPLE,
    raft::{build_raft_cluster, wait_until_rafts_ready, Raft},
    utils::load_peers,
};
use integration_test_runner::run_in_;

#[run_in_(container)]
#[tokio::test(flavor = "multi_thread", worker_threads = 10)]
pub async fn test_static_bootstrap() {
    let (tx_raft, rx_raft) = mpsc::channel::<(u64, Raft)>();

    let peers = load_peers(THREE_NODE_EXAMPLE).await.unwrap();
    let _raft_tasks = tokio::spawn(build_raft_cluster(tx_raft, peers.clone()));
    let mut rafts = wait_until_rafts_ready(None, rx_raft, 3).await;

    sleep(Duration::from_secs(1)).await;

    for (_, raft) in rafts.iter_mut() {
        raft.quit().await.expect("Failed to quit raft node");
    }
}
