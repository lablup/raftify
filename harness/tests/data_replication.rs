use harness::{constant::THREE_NODE_EXAMPLE, raft_server::run_rafts, utils::load_peers};

#[tokio::test]
pub async fn test_data_replication() {
    let peers = load_peers(THREE_NODE_EXAMPLE).await.unwrap();
    run_rafts(peers);
}
