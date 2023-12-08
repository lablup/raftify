use serde::Deserialize;
use std::fs;
use std::net::SocketAddr;
use std::path::Path;
use toml;

use raftify::{Config, Peers, RaftConfig};

pub fn build_config() -> Config {
    let mut cfg = Config::default();
    cfg.log_dir = "./logs".to_owned();
    cfg.compacted_log_dir = "./logs".to_owned();

    let mut raft_cfg = RaftConfig::default();
    raft_cfg.election_tick = 10;
    raft_cfg.heartbeat_tick = 3;

    cfg.raft_config = raft_cfg;
    return cfg;
}
