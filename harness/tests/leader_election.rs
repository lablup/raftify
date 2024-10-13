use raftify::{raft::StateRole, StableStorage};
use std::{process::exit, sync::mpsc};
use pretty_assertions::assert_eq;

use harness::{
    constant::{FIVE_NODE_EXAMPLE, THREE_NODE_EXAMPLE},
    raft::{build_raft_cluster, wait_until_rafts_ready, Raft},
    test_environment::{prepare_test_environment, TestEnvironment},
    utils::{gather_rafts_when_leader_elected, load_peers},
};

// Forcefully terminate the leader process as many times as the value of `election_iteration_cnt` and,
// Check if the leader election is properly performed.
async fn run_leader_election_test(
    test_environment: TestEnvironment,
    example: &str,
    election_iteration_cnt: u64,
) {
    let mut raft_list: Vec<u64> = match example {
        FIVE_NODE_EXAMPLE => vec![1, 2, 3, 4, 5],
        THREE_NODE_EXAMPLE => vec![1, 2, 3],
        _ => panic!("Unexpected case"),
    };

    let (tx_raft, rx_raft) = mpsc::channel::<(u64, Raft)>();
    let peers = load_peers(&test_environment.loopback_address, example)
        .await
        .unwrap();
    let _raft_tasks: tokio::task::JoinHandle<Result<(), raftify::Error>> =
        tokio::spawn(build_raft_cluster(
            tx_raft,
            test_environment.base_storage_path.clone(),
            peers.clone(),
        ));

    let mut rafts = wait_until_rafts_ready(None, rx_raft, raft_list.len()).await;
    let mut ex_term = 1;

    for _ in 0..election_iteration_cnt {
        let leader_id = rafts
            .get(&raft_list[0])
            .unwrap()
            .get_leader_id()
            .await
            .unwrap();
        let leader_raft = rafts.get_mut(&leader_id).unwrap();

        leader_raft.leave().await.unwrap();
        rafts.remove(&leader_id);
        raft_list.remove(raft_list.iter().position(|x| *x == leader_id).unwrap());

        let arbitrary_follower_id = raft_list[0];
        let follower_raft = rafts.get(&arbitrary_follower_id).unwrap().clone();

        let all_rafts = gather_rafts_when_leader_elected(&rafts).await;

        let candidates = all_rafts.get(&StateRole::Candidate).unwrap();

        let current_term = follower_raft
            .raft_node
            .storage()
            .await
            .unwrap()
            .hard_state()
            .unwrap()
            .get_term();

        assert!(!raft_list.contains(&leader_id));

        // Term must be increased after leader election
        let new_leader_id = rafts
            .get(&arbitrary_follower_id)
            .unwrap()
            .get_leader_id()
            .await
            .unwrap();

        assert!(candidates.get(&new_leader_id).is_some());
        assert!(current_term > ex_term);

        ex_term = current_term;

        assert_eq!(
            raft_list.len(),
            follower_raft.get_cluster_size().await.unwrap()
        );
    }

    for raft_id in raft_list {
        rafts.get_mut(&raft_id).unwrap().quit().await.unwrap();
    }
    exit(0);
}

#[tokio::test]
pub async fn test_leader_election_in_five_node_cluster() {
    run_leader_election_test(
        prepare_test_environment(stringify!(test_leader_election_in_five_node_cluster)),
        FIVE_NODE_EXAMPLE,
        2,
    )
    .await;
}

#[tokio::test]
pub async fn test_leader_election_in_three_node_cluster() {
    run_leader_election_test(
        prepare_test_environment(stringify!(test_leader_election_in_three_node_cluster)),
        THREE_NODE_EXAMPLE,
        1,
    )
    .await;
}
