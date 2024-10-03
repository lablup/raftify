use std::{sync::mpsc, time::Duration};
use tokio::time::{sleep, timeout};

use harness::{
    constant::{FIVE_NODE_EXAMPLE, THREE_NODE_EXAMPLE},
    raft::{build_raft_cluster, wait_until_rafts_ready, Raft},
    utils::{
        cleanup_storage, kill_previous_raft_processes, load_peers,
        wait_for_until_cluster_size_decrease, wait_for_until_cluster_size_increase,
    },
};

#[tokio::test]
pub async fn test_leader_election_in_three_node_example() {
    cleanup_storage("./logs");
    kill_previous_raft_processes();

    let (tx_raft, rx_raft) = mpsc::channel::<(u64, Raft)>();

    let peers = load_peers(THREE_NODE_EXAMPLE).await.unwrap();
    let _raft_tasks = tokio::spawn(build_raft_cluster(tx_raft, peers.clone()));
    sleep(Duration::from_secs(1)).await;

    let mut rafts = wait_until_rafts_ready(None, rx_raft, 3).await;

    let raft_1 = rafts.get_mut(&1).unwrap();

    wait_for_until_cluster_size_increase(raft_1.clone(), 3).await;

    sleep(Duration::from_secs(1)).await;

    raft_1.leave().await.expect("Failed to leave");

    sleep(Duration::from_secs(2)).await;

    let raft_2 = rafts.get_mut(&2).unwrap();

    wait_for_until_cluster_size_decrease(raft_2.clone(), 2).await;

    let leader_id = raft_2.get_leader_id().await.unwrap();

    let timer = timeout(Duration::from_secs(5), async {
        while leader_id == 0 {
            sleep(Duration::from_secs(1)).await;
        }
    })
    .await;

    assert!(timer.is_ok(), "Actual leader_id: {}", leader_id);
    assert!(
        [2, 3].contains(&leader_id),
        "Actual leader_id: {}",
        leader_id
    );

    raft_2.quit().await.expect("Failed to quit");
    let raft_3 = rafts.get_mut(&3).unwrap();
    raft_3.quit().await.expect("Failed to quit");
}

// TODO: Fix this test.
#[tokio::test]
#[ignore]
pub async fn test_leader_election_in_five_node_example() {
    cleanup_storage("./logs");
    kill_previous_raft_processes();

    let (tx_raft, rx_raft) = mpsc::channel::<(u64, Raft)>();
    let peers = load_peers(FIVE_NODE_EXAMPLE).await.unwrap();
    let _raft_tasks = tokio::spawn(build_raft_cluster(tx_raft, peers.clone()));

    sleep(Duration::from_secs(1)).await;

    let mut rafts = wait_until_rafts_ready(None, rx_raft, 5).await;

    let raft_1 = rafts.get_mut(&1).unwrap();

    wait_for_until_cluster_size_increase(raft_1.clone(), 5).await;

    sleep(Duration::from_secs(1)).await;

    raft_1.leave().await.expect("Failed to leave");

    let raft_2 = rafts.get_mut(&2).unwrap();

    wait_for_until_cluster_size_decrease(raft_2.clone(), 4).await;

    sleep(Duration::from_secs(2)).await;

    let leader_id = raft_2.get_leader_id().await.unwrap();

    assert!(
        [2, 3, 4, 5].contains(&leader_id),
        "Actual leader_id: {}",
        leader_id
    );

    let leader_raft = rafts.get_mut(&leader_id).unwrap();
    leader_raft.leave().await.expect("Failed to leave");

    let mut remaining_nodes = vec![2, 3, 4, 5];
    if let Some(pos) = remaining_nodes.iter().position(|&x| x == leader_id) {
        remaining_nodes.remove(pos);
    }

    let raft_k = rafts.get_mut(&remaining_nodes[0]).unwrap();

    wait_for_until_cluster_size_decrease(raft_k.clone(), 3).await;
    sleep(Duration::from_secs(2)).await;

    let leader_id = raft_k.get_leader_id().await.unwrap();

    assert!(leader_id != 0);
    assert_eq!(raft_k.get_cluster_size().await.unwrap(), 3);

    sleep(Duration::from_secs(2)).await;

    for id in remaining_nodes {
        let raft = rafts.get_mut(&id).unwrap();
        raft.quit().await.expect("Failed to quit the raft node");
    }
}
