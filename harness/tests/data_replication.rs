use std::time::Duration;

use harness::{
    constant::THREE_NODE_EXAMPLE,
    raft_server::{run_rafts, RAFTS},
    state_machine::LogEntry,
    utils::{load_peers, wait_for_until_cluster_size_increase},
};
use raftify::AbstractLogEntry;
use tokio::time::sleep;

#[tokio::test]
pub async fn test_data_replication() {
    let peers = load_peers(THREE_NODE_EXAMPLE).await.unwrap();
    let raft_tasks = tokio::spawn(run_rafts(peers));
    sleep(Duration::from_secs(1)).await;

    let rafts = RAFTS.lock().unwrap();
    let raft_1 = rafts.get(1).unwrap();

    println!("Waiting for cluster size to increase to 3...");
    wait_for_until_cluster_size_increase(raft_1.clone(), 3).await;
    println!("Waiting exited!");

    let entry = LogEntry::Insert {
        key: 1,
        value: "test".to_string(),
    }
    .encode()
    .unwrap();
    raft_1.mailbox().send(entry).await.unwrap();

    let result = tokio::join!(raft_tasks);
}
