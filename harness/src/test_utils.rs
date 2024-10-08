use std::{
    collections::{HashMap, HashSet},
    process::exit,
    sync::mpsc,
};

use raftify::{raft::StateRole, StableStorage};

use crate::{
    constant::{FIVE_NODE_EXAMPLE, THREE_NODE_EXAMPLE},
    raft::{build_raft_cluster, wait_until_rafts_ready, Raft},
    utils::load_peers,
};

// Forcefully terminate the leader process as many times as the value of `election_iteration_cnt` and,
// Check if the leader election is properly performed.
pub async fn run_leader_election_test(example: &str, election_iteration_cnt: u64) {
    let mut raft_list: Vec<u64> = match example {
        FIVE_NODE_EXAMPLE => vec![1, 2, 3, 4, 5],
        THREE_NODE_EXAMPLE => vec![1, 2, 3],
        _ => panic!("Unexpected case"),
    };

    let (tx_raft, rx_raft) = mpsc::channel::<(u64, Raft)>();
    let peers = load_peers(example).await.unwrap();
    let _raft_tasks = tokio::spawn(build_raft_cluster(tx_raft, peers.clone()));

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

/// Collects the IDs and roles of all Raft nodes during the latest term.
/// This function continuously checks the nodes until a leader is elected or the term changes.
/// It then returns a set containing each node's ID and its current role.
///
/// # Note
///
/// It may potentially hang if a leader does not emerge due to lack of quorum.
/// ```
pub async fn gather_rafts_when_leader_elected(
    rafts: &HashMap<u64, Raft>,
) -> HashMap<StateRole, HashSet<u64>> {
    let mut result: HashMap<StateRole, HashSet<u64>> = HashMap::from([
        (StateRole::Follower, HashSet::new()),
        (StateRole::Candidate, HashSet::new()),
        (StateRole::Leader, HashSet::new()),
        (StateRole::PreCandidate, HashSet::new()),
    ]);

    let mut current_term = 1;
    let mut leader_elected = false;

    while !leader_elected {
        for (id, raft) in rafts.iter() {
            let (term, state_role) = (
                raft.raft_node
                    .storage()
                    .await
                    .unwrap()
                    .hard_state()
                    .unwrap()
                    .get_term(),
                unsafe { raft.get_raw_node().await.unwrap().lock().await.raft.state },
            );

            if current_term > term {
                continue;
            }
            if current_term < term {
                result.values_mut().for_each(|v| v.clear());
                current_term = term;
            }

            result.get_mut(&state_role).unwrap().insert(*id);

            if state_role == StateRole::Leader {
                leader_elected = true;
            }
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use raftify::{raft::StateRole, HeedStorage, Raft as Raft_};

    use crate::{
        state_machine::{HashStore, LogEntry},
        test_utils::gather_rafts_when_leader_elected,
        utils::{cleanup_storage, kill_previous_raft_processes, load_peers},
    };

    pub type Raft = Raft_<LogEntry, HeedStorage, HashStore>;

    #[tokio::test]
    async fn test_gather_rafts_when_leader_elected() {
        use crate::constant::FIVE_NODE_EXAMPLE;
        use crate::raft::{build_raft_cluster, wait_until_rafts_ready};
        use std::sync::mpsc;

        cleanup_storage("./logs");
        kill_previous_raft_processes();
        let (tx_raft, rx_raft) = mpsc::channel::<(u64, Raft)>();
        let peers = load_peers(FIVE_NODE_EXAMPLE).await.unwrap();
        let _raft_tasks = tokio::spawn(build_raft_cluster(tx_raft, peers.clone()));

        let rafts = wait_until_rafts_ready(None, rx_raft, 5).await;

        let all_rafts = gather_rafts_when_leader_elected(&rafts).await;

        let leaders = all_rafts.get(&StateRole::Leader).unwrap();
        let followers = all_rafts.get(&StateRole::Follower).unwrap();
        let candidates = all_rafts.get(&StateRole::Candidate).unwrap();
        let precandidates = all_rafts.get(&StateRole::PreCandidate).unwrap();

        assert_eq!(1, leaders.len());
        assert_eq!(4, followers.len());
        assert_eq!(0, candidates.len());
        assert_eq!(0, precandidates.len());

        std::process::exit(0);
    }
}
