use raftify::{Config, RaftConfig};

pub fn build_config() -> Config {
    let mut cfg = Config::default();
    cfg.log_dir = "./logs".to_owned();

    cfg.save_compacted_logs = true;
    cfg.compacted_log_dir = "./logs".to_owned();
    cfg.compacted_log_size_threshold = 1024 * 1024 * 1024;

    let mut raft_cfg = RaftConfig::default();
    raft_cfg.election_tick = 10;
    raft_cfg.heartbeat_tick = 3;

    cfg.raft_config = raft_cfg;
    return cfg;
}
