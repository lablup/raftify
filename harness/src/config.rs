use std::fs;
use serde::Deserialize;
use raftify::{Config, Peers, RaftConfig};  // ConfigBuilder 삭제
use crate::utils::{ensure_directory_exist, get_storage_path};

#[derive(Deserialize)]
struct NodeConfig {
    id: u64,
}

#[derive(Deserialize)]
struct RaftConfigToml {
    election_tick: u64,
    heartbeat_tick: u64,
    omit_heartbeat_log: bool,
}

#[derive(Deserialize)]
struct StorageConfig {
    log_dir: String,
    compacted_log_size_threshold: u64,
}

#[derive(Deserialize)]
struct ConfigToml {
    node: NodeConfig,
    raft: RaftConfigToml,
    storage: StorageConfig,
}

// TOML 파일에서 설정을 읽어오는 함수
pub fn load_config() -> ConfigToml {
    let config_content = fs::read_to_string("config.toml").expect("Failed to read config.toml");
    toml::from_str(&config_content).expect("Failed to parse config.toml")
}

// ConfigBuilder 없이 Config 직접 생성
pub fn build_config(initial_peers: Option<Peers>) -> Config {
    // TOML 파일에서 설정을 읽음
    let config = load_config();

    // RaftConfig 생성
    let raft_config = RaftConfig {
        id: config.node.id,
        election_tick: config.raft.election_tick as usize,
        heartbeat_tick: config.raft.heartbeat_tick as usize,
        omit_heartbeat_log: config.raft.omit_heartbeat_log,
        ..Default::default()
    };

    let storage_path = get_storage_path(&config.storage.log_dir, config.node.id);
    ensure_directory_exist(&storage_path).expect("Failed to create storage directory");

    Config {
        raft_config,
        log_dir: storage_path.clone(),
        save_compacted_logs: true,
        compacted_log_dir: storage_path,
        compacted_log_size_threshold: config.storage.compacted_log_size_threshold,
        initial_peers,  // 초기 피어 설정
        ..Default::default()
    }
}
