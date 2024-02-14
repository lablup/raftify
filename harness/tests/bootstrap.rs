use std::{sync::mpsc, time::Duration};
use tokio::time::sleep;

use harness::{
    constant::{ONE_NODE_EXAMPLE, RAFT_ADDRS, THREE_NODE_EXAMPLE},
    raft::{build_raft_cluster, spawn_extra_node, wait_until_rafts_ready, Raft},
    utils::{kill_previous_raft_processes, load_peers, wait_for_until_cluster_size_increase},
};

#[tokio::test]
pub async fn test_static_bootstrap() {
    kill_previous_raft_processes();
    let (raft_tx, raft_rx) = mpsc::channel::<(u64, Raft)>();

    let peers = load_peers(THREE_NODE_EXAMPLE).await.unwrap();
    let _raft_tasks = tokio::spawn(build_raft_cluster(raft_tx, peers.clone()));
    let mut rafts = wait_until_rafts_ready(None, raft_rx, 3).await;

    sleep(Duration::from_secs(1)).await;

    let raft_1 = rafts.get(&1).unwrap();

    wait_for_until_cluster_size_increase(raft_1.clone(), 3).await;

    for (_, raft) in rafts.iter_mut() {
        raft.raft_node.quit().await;
    }

    sleep(Duration::from_secs(1)).await;
}

#[tokio::test]
pub async fn test_dynamic_bootstrap() {
    kill_previous_raft_processes();
    let (raft_tx, raft_rx) = mpsc::channel::<(u64, Raft)>();

    let peers = load_peers(ONE_NODE_EXAMPLE).await.unwrap();
    let _raft_tasks = tokio::spawn(build_raft_cluster(raft_tx.clone(), peers.clone()));

    sleep(Duration::from_secs(1)).await;

    tokio::spawn(spawn_extra_node(
        raft_tx.clone(),
        "127.0.0.1:60062",
        RAFT_ADDRS[0],
    ))
    .await
    .unwrap()
    .unwrap();

    tokio::spawn(spawn_extra_node(
        raft_tx.clone(),
        "127.0.0.1:60063",
        RAFT_ADDRS[0],
    ))
    .await
    .unwrap()
    .unwrap();

    let mut rafts = wait_until_rafts_ready(None, raft_rx, 3).await;

    let raft_1 = rafts.get(&1).unwrap();

    wait_for_until_cluster_size_increase(raft_1.clone(), 3).await;

    for (_, raft) in rafts.iter_mut() {
        raft.raft_node.quit().await;
    }
}
