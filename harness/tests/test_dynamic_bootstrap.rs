use integration_test_runner::run_in_;
use std::{process::exit, sync::mpsc, time::Duration};
use tokio::time::sleep;

use harness::{
    constant::{ONE_NODE_EXAMPLE, RAFT_ADDRS},
    raft::{build_raft_cluster, spawn_and_join_extra_node, wait_until_rafts_ready, Raft},
    utils::load_peers,
};

#[run_in_(container)]
#[tokio::test(flavor = "multi_thread", worker_threads = 10)]
pub async fn test_dynamic_bootstrap() {
    let (tx_raft, rx_raft) = mpsc::channel::<(u64, Raft)>();

    let peers = load_peers(ONE_NODE_EXAMPLE).await.unwrap();
    let _raft_tasks = tokio::spawn(build_raft_cluster(tx_raft.clone(), peers.clone()));

    sleep(Duration::from_secs(1)).await;

    tokio::spawn(spawn_and_join_extra_node(
        tx_raft.clone(),
        "127.0.0.1:60062",
        RAFT_ADDRS[0],
    ))
    .await
    .unwrap()
    .unwrap();

    tokio::spawn(spawn_and_join_extra_node(
        tx_raft.clone(),
        "127.0.0.1:60063",
        RAFT_ADDRS[0],
    ))
    .await
    .unwrap()
    .unwrap();

    let mut rafts = wait_until_rafts_ready(None, rx_raft, 3).await;

    for (_, raft) in rafts.iter_mut() {
        raft.quit().await.expect("Failed to quit raft node");
    }
    exit(0);
}
