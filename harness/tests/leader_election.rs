use std::time::Duration;
use tokio::time::sleep;

use harness::{
    constant::THREE_NODE_EXAMPLE,
    raft_server::{handle_bootstrap, run_rafts, RAFTS},
    utils::{
        load_peers, wait_for_until_cluster_size_decrease, wait_for_until_cluster_size_increase,
    },
};

#[tokio::test]
pub async fn test_leader_election() {
    let peers = load_peers(THREE_NODE_EXAMPLE).await.unwrap();
    let _raft_tasks = tokio::spawn(run_rafts(peers.clone()));

    sleep(Duration::from_secs(1)).await;

    handle_bootstrap(peers).await.unwrap();

    let mut rafts = RAFTS.lock().unwrap();
    let raft_1 = rafts.get_mut(&1).unwrap();

    wait_for_until_cluster_size_increase(raft_1.clone(), 3).await;

    sleep(Duration::from_secs(1)).await;

    raft_1.raft_node.leave().await;

    sleep(Duration::from_secs(3)).await;

    let raft_2 = rafts.get_mut(&2).unwrap();

    wait_for_until_cluster_size_decrease(raft_2.clone(), 2).await;

    let leader_id = raft_2.raft_node.get_leader_id().await;
    assert!(leader_id == 2 || leader_id == 3);

    raft_2.raft_node.quit().await;
    let raft_3 = rafts.get_mut(&3).unwrap();
    raft_3.raft_node.quit().await;
}
