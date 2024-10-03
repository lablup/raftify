use std::fs;
use serde::{Deserialize, Serialize};
use crate::{error::Error, raft::Config as RaftConfig, InitialRole, Peers, Result};

// NOTE: This TLS Config is a type commonly used in both RaftServer and RaftClient.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TlsConfig {
    pub cert_path: Option<String>,
    pub key_path: Option<String>,
    pub ca_cert_path: Option<String>,
    pub domain_name: Option<String>,
}

#[derive(Clone)]
pub struct Config {
    pub raft_config: RaftConfig,
    pub log_dir: String,
    pub save_compacted_logs: bool,
    pub compacted_log_dir: String,
    pub compacted_log_size_threshold: u64,
    pub(crate) tick_interval: f32,
    pub(crate) lmdb_map_size: u64,
    pub(crate) bootstrap_from_snapshot: bool,
    pub(crate) cluster_id: String,
    pub(crate) conf_change_request_timeout: f32,
    pub initial_peers: Option<Peers>,
    pub(crate) snapshot_interval: Option<f32>,
    pub(crate) client_tls_config: Option<TlsConfig>,
    pub(crate) server_tls_config: Option<TlsConfig>,
}

#[derive(Deserialize)]
struct RaftConfigToml {
    election_tick: u64,
    heartbeat_tick: u64,
    omit_heartbeat_log: bool,
    applied: u64,
    max_size_per_msg: u64,
    max_inflight_msgs: u64,
    check_quorum: bool,
    pre_vote: bool,
    min_election_tick: u64,
    max_election_tick: u64,
    read_only_option: String,
    skip_bcast_commit: bool,
    batch_append: bool,
    priority: i32,
    max_uncommitted_size: u64,
    max_committed_size_per_ready: u64,
}

#[derive(Deserialize)]
struct StorageConfig {
    log_dir: String,
    compacted_log_size_threshold: u64,
}

#[derive(Deserialize)]
struct ClusterConfig {
    cluster_id: String,
    tick_interval: f32,
    lmdb_map_size: u64,
    conf_change_request_timeout: f32,
}

#[derive(Deserialize)]
struct TlsConfigToml {
    cert_path: Option<String>,
    key_path: Option<String>,
    ca_cert_path: Option<String>,
    domain_name: Option<String>,
}

#[derive(Deserialize)]
struct NodeConfig {
    id: u64,
}

#[derive(Deserialize)]
struct ConfigToml {
    node: NodeConfig,
    raft: RaftConfigToml,
    storage: StorageConfig,
    cluster: ClusterConfig,
    tls: TlsConfigToml,
}

pub fn load_config() -> ConfigToml {
    let config_content = fs::read_to_string("config.toml").expect("Failed to read config.toml");
    toml::from_str(&config_content).expect("Failed to parse config.toml")
}

pub fn build_config(initial_peers: Option<Peers>) -> Config {
    let config = load_config();

    let raft_config = RaftConfig {
        id: config.node.id,
        election_tick: config.raft.election_tick as usize,
        heartbeat_tick: config.raft.heartbeat_tick as usize,
        omit_heartbeat_log: config.raft.omit_heartbeat_log,
        applied: config.raft.applied,
        max_size_per_msg: config.raft.max_size_per_msg,
        max_inflight_msgs: config.raft.max_inflight_msgs as usize,
        check_quorum: config.raft.check_quorum,
        pre_vote: config.raft.pre_vote,
        min_election_tick: config.raft.min_election_tick as usize,
        max_election_tick: config.raft.max_election_tick as usize,
        read_only_option: config.raft.read_only_option.parse().unwrap(),
        skip_bcast_commit: config.raft.skip_bcast_commit,
        batch_append: config.raft.batch_append,
        priority: config.raft.priority as i64,
        max_uncommitted_size: config.raft.max_uncommitted_size,
        max_committed_size_per_ready: config.raft.max_committed_size_per_ready,
        max_apply_unpersisted_log_limit: 0,
    };

    let tls_config = TlsConfig {
        cert_path: config.tls.cert_path.clone(),
        key_path: config.tls.key_path.clone(),
        ca_cert_path: config.tls.ca_cert_path.clone(),
        domain_name: config.tls.domain_name.clone(),
    };

    Config {
        raft_config,
        log_dir: config.storage.log_dir.clone(),
        save_compacted_logs: true,
        compacted_log_dir: config.storage.log_dir.clone(),
        compacted_log_size_threshold: config.storage.compacted_log_size_threshold,
        tick_interval: config.cluster.tick_interval,
        lmdb_map_size: config.cluster.lmdb_map_size,
        bootstrap_from_snapshot: false,
        cluster_id: config.cluster.cluster_id.clone(),
        conf_change_request_timeout: config.cluster.conf_change_request_timeout,
        initial_peers,
        snapshot_interval: None,
        client_tls_config: Some(tls_config.clone()),
        server_tls_config: Some(tls_config),
    }
}
