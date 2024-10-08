use harness::{constant::FIVE_NODE_EXAMPLE, test_utils::run_leader_election_test};
use integration_test_runner::run_in_;

#[run_in_(container)]
#[tokio::test(flavor = "multi_thread", worker_threads = 10)]
pub async fn test_leader_election_in_five_node_cluster() {
    run_leader_election_test(FIVE_NODE_EXAMPLE, 2).await;
}
