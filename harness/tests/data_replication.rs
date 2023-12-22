use std::time::Duration;

use harness::{
    constant::{RAFT_ADDRS, THREE_NODE_EXAMPLE},
    raft_server::{handle_bootstrap, run_rafts, spawn_extra_node, RAFTS},
    state_machine::LogEntry,
    utils::{load_peers, wait_for_until_cluster_size_increase},
};
use raftify::AbstractLogEntry;
use tokio::time::sleep;

#[tokio::test]
pub async fn test_data_replication() {
    {
        let peers = load_peers(THREE_NODE_EXAMPLE).await.unwrap();
        let _raft_tasks = tokio::spawn(run_rafts(peers.clone()));

        sleep(Duration::from_secs(1)).await;

        handle_bootstrap(peers).await.unwrap();

        let mut rafts = RAFTS.lock().unwrap();
        let raft_1 = rafts.get(&1).unwrap();

        wait_for_until_cluster_size_increase(raft_1.clone(), 3).await;

        let entry = LogEntry::Insert {
            key: 1,
            value: "test".to_string(),
        }
        .encode()
        .unwrap();

        raft_1.raft_node.propose(entry).await;

        sleep(Duration::from_secs(1)).await;

        // Data should be replicated to all nodes.
        for (_, raft) in rafts.iter_mut() {
            let store = raft.raft_node.store().await;
            let store_lk = store.0.read().unwrap();
            assert_eq!(store_lk.get(&1).unwrap(), "test");
        }

        sleep(Duration::from_secs(1)).await;
    }

    tokio::spawn(spawn_extra_node(RAFT_ADDRS[0], "127.0.0.1:60064"));
    sleep(Duration::from_secs(1)).await;

    {
        let mut rafts = RAFTS.lock().unwrap();
        let raft_1 = rafts.get(&1).unwrap();
        wait_for_until_cluster_size_increase(raft_1.clone(), 4).await;
        let raft_4 = rafts.get(&4).unwrap();
        let store = raft_4.raft_node.store().await;
        let store_lk = store.0.read().unwrap();

        // Data should be replicated to new joined node.
        assert_eq!(store_lk.get(&1).unwrap(), "test");

        for (_, raft) in rafts.iter_mut() {
            raft.raft_node.quit(true).await;
        }
    }
}
