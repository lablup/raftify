use crate::utils::{ensure_directory_exist, get_storage_path};
use raftify::{Config, ConfigBuilder, Peers, RaftConfig};

pub fn build_config(node_id: u64, base_storage_path: &str, initial_peers: Option<Peers>) -> Config {
    let raft_config = RaftConfig {
        id: node_id,
        election_tick: 10,
        heartbeat_tick: 3,
        omit_heartbeat_log: true,
        ..Default::default()
    };

    let storage_path = get_storage_path(base_storage_path, node_id);
    ensure_directory_exist(&storage_path).expect("Failed to create storage directory");

    let config_builder = ConfigBuilder::new()
        .log_dir(storage_path.clone())
        .save_compacted_logs(true)
        .compacted_log_dir(storage_path)
        .compacted_log_size_threshold(1024 * 1024 * 1024)
        .raft_config(raft_config);

    let config_builder = if let Some(peers) = initial_peers {
        config_builder.initial_peers(peers)
    } else {
        config_builder
    };

    config_builder.build()
}
