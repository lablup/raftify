use raftify::AbstractLogEntry;
use std::{sync::mpsc, time::Duration};
use tokio::time::sleep;

use harness::{
    constant::{RAFT_PORTS, THREE_NODE_EXAMPLE},
    raft::{build_raft_cluster, spawn_and_join_extra_node, wait_until_rafts_ready, Raft},
    state_machine::LogEntry,
    test_enviorment_utils::get_test_environment,
    utils::load_peers,
};

#[tokio::test]
pub async fn test_data_replication() {
    let test_environment = get_test_environment(stringify!(test_data_replication));

    let peers = load_peers(&test_environment.loopback_address, THREE_NODE_EXAMPLE)
        .await
        .unwrap();
    let (tx_raft, rx_raft) = mpsc::channel::<(u64, Raft)>();

    let _raft_tasks = tokio::spawn(build_raft_cluster(
        tx_raft,
        test_environment.base_storage_path.clone(),
        peers.clone(),
    ));
    sleep(Duration::from_secs(1)).await;

    let mut rafts = wait_until_rafts_ready(None, rx_raft, 3).await;

    let raft_1 = rafts.get(&1).unwrap();

    let entry = LogEntry::Insert {
        key: 1,
        value: "test".to_string(),
    }
    .encode()
    .unwrap();

    raft_1.propose(entry).await.unwrap();

    sleep(Duration::from_secs(3)).await;

    // Data should be replicated to all nodes.
    for (_, raft) in rafts.iter_mut() {
        let store = raft.state_machine().await.unwrap();
        let store_lk = store.0.read().unwrap();
        assert_eq!(store_lk.get(&1).unwrap(), "test");
    }

    sleep(Duration::from_secs(1)).await;

    let (tx_raft, rx_raft) = mpsc::channel::<(u64, Raft)>();
    tokio::spawn(spawn_and_join_extra_node(
        tx_raft,
        test_environment.loopback_address.clone() + ":60064",
        test_environment.loopback_address.clone() + ":" + &RAFT_PORTS[0].to_string(),
        test_environment.base_storage_path.clone(),
    ));
    sleep(Duration::from_secs(1)).await;

    let mut rafts = wait_until_rafts_ready(Some(rafts), rx_raft, 4).await;
    sleep(Duration::from_secs(1)).await;

    let raft_4 = rafts.get(&4).unwrap();
    let store = raft_4.state_machine().await.unwrap();
    let store_lk = store.0.read().unwrap();

    // Data should be replicated to new member.
    assert_eq!(store_lk.get(&1).unwrap(), "test");
    std::mem::drop(store_lk);

    let raft_1 = rafts.get(&1).unwrap();

    let new_entry = LogEntry::Insert {
        key: 2,
        value: "test2".to_string(),
    }
    .encode()
    .unwrap();

    raft_1.propose(new_entry).await.unwrap();

    // New entry data should be replicated to all nodes including new member.
    for (_, raft) in rafts.iter() {
        let store = raft.state_machine().await.unwrap();
        let store_lk = store.0.read().unwrap();
        assert_eq!(store_lk.get(&2).unwrap(), "test2");
    }

    for (_, raft) in rafts.iter_mut() {
        raft.quit().await.expect("Failed to quit the raft node");
    }
}
