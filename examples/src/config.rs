use raftify::{Config, RaftConfig};

pub fn build_config() -> Config {
    let raft_config = RaftConfig {
        election_tick: 10,
        heartbeat_tick: 3,
        ..Default::default()
    };

    Config {
        log_dir: "./logs".to_owned(),
        save_compacted_logs: true,
        compacted_log_dir: "./logs".to_owned(),
        compacted_log_size_threshold: 1024 * 1024 * 1024,
        raft_config,
        ..Default::default()
    }
}
