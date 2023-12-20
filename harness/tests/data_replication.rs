use harness::{constant::THREE_NODE_EXAMPLE, utils::load_peers, raft_server::run_rafts};

#[tokio::test]
pub async fn test_data_replication() {
    let peers = load_peers(THREE_NODE_EXAMPLE).await.unwrap();
    run_rafts(peers);
}
