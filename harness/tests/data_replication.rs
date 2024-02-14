use raftify::AbstractLogEntry;
use std::{sync::mpsc, time::Duration};
use tokio::time::sleep;

use harness::{
    constant::{RAFT_ADDRS, THREE_NODE_EXAMPLE},
    raft::{build_raft_cluster, spawn_extra_node, wait_until_rafts_ready, Raft},
    state_machine::LogEntry,
    utils::{kill_previous_raft_processes, load_peers, wait_for_until_cluster_size_increase},
};

#[tokio::test]
pub async fn test_data_replication() {
    kill_previous_raft_processes();

    let peers = load_peers(THREE_NODE_EXAMPLE).await.unwrap();
    let (raft_tx, raft_rx) = mpsc::channel::<(u64, Raft)>();

    let _raft_tasks = tokio::spawn(build_raft_cluster(raft_tx, peers.clone()));
    sleep(Duration::from_secs(1)).await;

    let mut rafts = wait_until_rafts_ready(None, raft_rx, 3).await;

    let raft_1 = rafts.get(&1).unwrap();
    wait_for_until_cluster_size_increase(raft_1.clone(), 3).await;

    let entry = LogEntry::Insert {
        key: 1,
        value: "test".to_string(),
    }
    .encode()
    .unwrap();

    raft_1.raft_node.propose(entry).await.unwrap();

    sleep(Duration::from_secs(1)).await;

    // Data should be replicated to all nodes.
    for (_, raft) in rafts.iter_mut() {
        let store = raft.raft_node.store().await;
        let store_lk = store.0.read().unwrap();
        assert_eq!(store_lk.get(&1).unwrap(), "test");
    }

    sleep(Duration::from_secs(1)).await;

    let (raft_tx, raft_rx) = mpsc::channel::<(u64, Raft)>();
    tokio::spawn(spawn_extra_node(raft_tx, "127.0.0.1:60064", RAFT_ADDRS[0]));
    sleep(Duration::from_secs(1)).await;

    let mut rafts = wait_until_rafts_ready(Some(rafts), raft_rx, 4).await;
    sleep(Duration::from_secs(1)).await;

    let raft_1 = rafts.get(&1).unwrap();
    wait_for_until_cluster_size_increase(raft_1.clone(), 4).await;

    let raft_4 = rafts.get(&4).unwrap();
    let store = raft_4.raft_node.store().await;
    let store_lk = store.0.read().unwrap();

    // Data should be replicated to new joined node.
    assert_eq!(store_lk.get(&1).unwrap(), "test");
    std::mem::drop(store_lk);

    let raft_1 = rafts.get(&1).unwrap();

    let new_entry = LogEntry::Insert {
        key: 2,
        value: "test2".to_string(),
    }
    .encode()
    .unwrap();

    raft_1.raft_node.propose(new_entry).await.unwrap();

    // New entry data should be replicated to all nodes including new joined node.
    for (_, raft) in rafts.iter() {
        // stop
        let store = raft.raft_node.store().await;
        let store_lk = store.0.read().unwrap();
        assert_eq!(store_lk.get(&2).unwrap(), "test2");
    }

    for (_, raft) in rafts.iter_mut() {
        raft.raft_node.quit().await;
    }
}
