use raftify::{Config, RaftConfig};

pub fn build_config(node_id: u64) -> Config {
    let raft_config = RaftConfig {
        id: node_id,
        election_tick: 10,
        heartbeat_tick: 3,
        ..Default::default()
    };

    Config {
        tick_interval: 0.2,
        log_dir: "./logs".to_owned(),
        save_compacted_logs: true,
        compacted_log_dir: "./logs".to_owned(),
        compacted_log_size_threshold: 1024 * 1024 * 1024,
        raft_config,
        ..Default::default()
    }
}
