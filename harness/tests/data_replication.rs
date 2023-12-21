use std::time::Duration;

use harness::{
    constant::THREE_NODE_EXAMPLE,
    raft_server::{bootstrap, run_rafts, RAFTS},
    state_machine::LogEntry,
    utils::{load_peers, wait_for_until_cluster_size_increase},
};
use raftify::AbstractLogEntry;
use tokio::time::sleep;

#[tokio::test]
pub async fn test_data_replication() {
    let peers = load_peers(THREE_NODE_EXAMPLE).await.unwrap();
    let _raft_tasks = tokio::spawn(run_rafts(peers.clone()));

    sleep(Duration::from_secs(1)).await;

    bootstrap(peers).await.unwrap();

    let mut rafts = RAFTS.lock().unwrap();
    let raft_1 = rafts.get(1).unwrap();

    println!("Waiting for cluster size to increase to 3...");
    wait_for_until_cluster_size_increase(raft_1.clone(), 3).await;

    let entry = LogEntry::Insert {
        key: 1,
        value: "test".to_string(),
    }
    .encode()
    .unwrap();

    raft_1.raft_node.propose(entry).await;

    // Data should be replicated to all nodes.

    for raft in rafts.iter_mut() {
        let store = raft.raft_node.store().await;
        let store_lk = store.0.read().unwrap();
        assert_eq!(store_lk.get(&1).unwrap(), "test");
    }

    sleep(Duration::from_secs(1)).await;

    for raft in rafts.iter_mut() {
        raft.raft_node.quit().await;
    }
}
