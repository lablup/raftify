use std::{sync::mpsc, time::Duration};
use raftify::StableStorage;
use tokio::time::{sleep, timeout};

use harness::{
    constant::{FIVE_NODE_EXAMPLE, THREE_NODE_EXAMPLE},
    raft::{build_raft_cluster, wait_until_rafts_ready, Raft},
    utils::{
        count_voter, kill_previous_raft_processes, load_peers, wait_for_until_cluster_size_decrease, wait_for_until_cluster_size_increase
    },
};

#[tokio::test]
pub async fn test_leader_election_in_three_node_example() {
    kill_previous_raft_processes();

    let (tx_raft, rx_raft) = mpsc::channel::<(u64, Raft)>();

    let peers = load_peers(THREE_NODE_EXAMPLE).await.unwrap();
    let _raft_tasks = tokio::spawn(build_raft_cluster(tx_raft, peers.clone()));
    sleep(Duration::from_secs(1)).await;

    let mut rafts = wait_until_rafts_ready(None, rx_raft, 3).await;

    let raft_1 = rafts.get_mut(&1).unwrap();

    wait_for_until_cluster_size_increase(raft_1.clone(), 3).await;

    sleep(Duration::from_secs(1)).await;

    raft_1.leave().await;

    sleep(Duration::from_secs(2)).await;

    let raft_2 = rafts.get_mut(&2).unwrap();

    wait_for_until_cluster_size_decrease(raft_2.clone(), 2).await;

    let leader_id = raft_2.get_leader_id().await;

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

    raft_2.quit().await;
    let raft_3 = rafts.get_mut(&3).unwrap();
    raft_3.quit().await;
}

// TODO: Fix this test.
#[tokio::test]
pub async fn test_leader_election_in_five_node_example() {
    // GIVEN
    kill_previous_raft_processes();

    let (tx_raft, rx_raft) = mpsc::channel::<(u64, Raft)>();
    let peers = load_peers(FIVE_NODE_EXAMPLE).await.unwrap();
    let _raft_tasks = tokio::spawn(build_raft_cluster(tx_raft, peers.clone()));
    let mut raft_list: Vec<u64> = vec![1, 2, 3, 4, 5];
    let mut current_node_cnt = raft_list.len();

    sleep(Duration::from_secs(1)).await;

    let mut rafts = wait_until_rafts_ready(None, rx_raft, 5).await;

    // WHEN
    for expected_term in 1..3 {
        let leader_id = rafts.get(&raft_list[0]).unwrap().get_leader_id().await;
        raft_list.remove(raft_list.iter().position(|x| *x == leader_id).unwrap());
        let arbitrary_follower_id = raft_list[0];

        let follower_raft = rafts.get(&arbitrary_follower_id).unwrap().clone();
        let leader_raft = rafts.get_mut(&leader_id).unwrap();

        leader_raft.leave().await;
        current_node_cnt -= 1;
        sleep(Duration::from_secs(1)).await;

        let follower_term = follower_raft
            .raft_node
            .storage()
            .await
            .hard_state()
            .unwrap()
            .get_term();

        wait_for_until_cluster_size_decrease(follower_raft.clone(), current_node_cnt).await;

        // THEN
        // Term must be increased after leader election
        assert!(leader_id != 0);
        assert!(!raft_list.contains(&leader_id));
        assert_eq!(follower_term, expected_term);
        assert_eq!(count_voter(&follower_raft).await, current_node_cnt);
        assert_eq!(current_node_cnt, follower_raft.get_cluster_size().await);
    }
    for raft_id in raft_list{
        rafts.get_mut(&raft_id).unwrap().quit().await
    }
}
