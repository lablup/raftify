use std::fmt;

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
    pub(crate) raft_config: RaftConfig,
    pub(crate) log_dir: String,
    pub(crate) save_compacted_logs: bool,
    pub(crate) compacted_log_dir: String,
    pub(crate) compacted_log_size_threshold: u64,
    pub(crate) tick_interval: f32,
    pub(crate) lmdb_map_size: u64,
    pub(crate) bootstrap_from_snapshot: bool,
    pub(crate) cluster_id: String,
    pub(crate) conf_change_request_timeout: f32,
    pub(crate) initial_peers: Option<Peers>,
    pub(crate) snapshot_interval: Option<f32>,
    pub(crate) client_tls_config: Option<TlsConfig>,
    pub(crate) server_tls_config: Option<TlsConfig>,
}

pub struct ConfigBuilder {
    config: Config,
    global_client_tls_config: Option<TlsConfig>,
}

impl ConfigBuilder {
    pub fn new() -> Self {
        Self {
            config: Config::default(),
            global_client_tls_config: None,
        }
    }

    pub fn log_dir(mut self, log_dir: String) -> Self {
        self.config.log_dir = log_dir;
        self
    }

    pub fn save_compacted_logs(mut self, save: bool) -> Self {
        self.config.save_compacted_logs = save;
        self
    }

    pub fn compacted_log_dir(mut self, dir: String) -> Self {
        self.config.compacted_log_dir = dir;
        self
    }

    pub fn compacted_log_size_threshold(mut self, threshold: u64) -> Self {
        self.config.compacted_log_size_threshold = threshold;
        self
    }

    pub fn raft_config(mut self, raft_config: RaftConfig) -> Self {
        self.config.raft_config = raft_config;
        self
    }

    pub fn tick_interval(mut self, interval: f32) -> Self {
        self.config.tick_interval = interval;
        self
    }

    pub fn lmdb_map_size(mut self, size: u64) -> Self {
        self.config.lmdb_map_size = size;
        self
    }

    pub fn cluster_id(mut self, cluster_id: String) -> Self {
        self.config.cluster_id = cluster_id;
        self
    }

    pub fn conf_change_request_timeout(mut self, timeout: f32) -> Self {
        self.config.conf_change_request_timeout = timeout;
        self
    }

    pub fn initial_peers(mut self, peers: Peers) -> Self {
        self.config.initial_peers = Some(peers);
        self
    }

    pub fn snapshot_interval(mut self, interval: f32) -> Self {
        self.config.snapshot_interval = Some(interval);
        self
    }

    pub fn server_tls_config(mut self, config: TlsConfig) -> Self {
        self.config.server_tls_config = Some(config);
        self
    }

    pub fn bootstrap_from_snapshot(mut self, v: bool) -> Self {
        self.config.bootstrap_from_snapshot = v;
        self
    }

    pub fn global_client_tls_config(mut self, config: TlsConfig) -> Self {
        self.global_client_tls_config = Some(config);
        self
    }

    pub fn build(mut self) -> Config {
        self.config.client_tls_config = self.global_client_tls_config.clone();

        if let Some(mut initial_peers) = self.config.initial_peers.clone() {
            if let Some(self_peer) = initial_peers.get_mut(&self.config.raft_config.id) {
                if self_peer.client_tls_config.is_none() {
                    self_peer.client_tls_config = self.global_client_tls_config.clone();
                }
            }
            self.config.initial_peers = Some(initial_peers);
        }

        self.config
    }
}

impl Config {
    pub fn validate(&self) -> Result<()> {
        if self.initial_peers.is_some() {
            let leaders = self
                .initial_peers
                .clone()
                .unwrap()
                .inner
                .into_iter()
                .filter(|(_, peer)| peer.initial_role == InitialRole::Leader)
                .map(|(key, _)| key)
                .collect::<Vec<_>>();

            if leaders.len() > 1 {
                return Err(Error::ConfigInvalid(
                    "initial_peers should contain at most 1 leaders".to_owned(),
                ));
            }
        }

        self.raft_config.validate()?;
        Ok(())
    }

    pub fn get_log_dir(&self) -> &str {
        &self.log_dir
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            raft_config: RaftConfig::default(),
            log_dir: String::from("./"),
            save_compacted_logs: false,
            compacted_log_dir: String::from("./"),
            compacted_log_size_threshold: 1024 * 1024 * 1024,
            tick_interval: 0.1,
            lmdb_map_size: 1024 * 1024 * 1024,
            cluster_id: String::from("default"),
            conf_change_request_timeout: 2.0,
            initial_peers: None,
            snapshot_interval: None,
            bootstrap_from_snapshot: false,
            client_tls_config: None,
            server_tls_config: None,
        }
    }
}

impl fmt::Debug for Config {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Config {{ \
                raft_config: {{ \
                    id: {id}, \
                    election_tick: {election_tick}, \
                    heartbeat_tick: {heartbeat_tick}, \
                    applied: {applied}, \
                    max_size_per_msg: {max_size_per_msg}, \
                    max_inflight_msgs: {max_inflight_msgs}, \
                    check_quorum: {check_quorum}, \
                    pre_vote: {pre_vote}, \
                    min_election_tick: {min_election_tick}, \
                    max_election_tick: {max_election_tick}, \
                    read_only_option: {read_only_option:?}, \
                    skip_bcast_commit: {skip_bcast_commit}, \
                    batch_append: {batch_append}, \
                    priority: {priority}, \
                    max_uncommitted_size: {max_uncommitted_size}, \
                    max_committed_size_per_ready: {max_committed_size_per_ready}, \
                }}, \
                log_dir: {log_dir}, \
                bootstrap_from_snapshot: {bootstrap_from_snapshot}, \
                save_compacted_logs: {save_compacted_logs}, \
                compacted_log_dir: {compacted_log_dir}, \
                compacted_log_size_threshold: {compacted_log_size_threshold}, \
                snapshot_interval: {snapshot_interval:?}, \
                tick_interval: {tick_interval}, \
                initial_peers: {initial_peers:?}, \
                lmdb_map_size: {lmdb_map_size}, \
                cluster_id: {cluster_id}, \
                conf_change_request_timeout: {conf_change_request_timeout}, \
                client_tls_config: {client_tls_config:?}, \
                server_tls_config: {server_tls_config:?}, \
            }}",
            id = self.raft_config.id,
            election_tick = self.raft_config.election_tick,
            heartbeat_tick = self.raft_config.heartbeat_tick,
            applied = self.raft_config.applied,
            max_size_per_msg = self.raft_config.max_size_per_msg,
            max_inflight_msgs = self.raft_config.max_inflight_msgs,
            check_quorum = self.raft_config.check_quorum,
            pre_vote = self.raft_config.pre_vote,
            min_election_tick = self.raft_config.min_election_tick,
            max_election_tick = self.raft_config.max_election_tick,
            read_only_option = self.raft_config.read_only_option,
            skip_bcast_commit = self.raft_config.skip_bcast_commit,
            batch_append = self.raft_config.batch_append,
            priority = self.raft_config.priority,
            max_uncommitted_size = self.raft_config.max_uncommitted_size,
            max_committed_size_per_ready = self.raft_config.max_committed_size_per_ready,
            log_dir = self.log_dir,
            save_compacted_logs = self.save_compacted_logs,
            compacted_log_dir = self.compacted_log_dir,
            compacted_log_size_threshold = self.compacted_log_size_threshold,
            snapshot_interval = self.snapshot_interval,
            tick_interval = self.tick_interval,
            lmdb_map_size = self.lmdb_map_size,
            initial_peers = self.initial_peers,
            cluster_id = self.cluster_id,
            conf_change_request_timeout = self.conf_change_request_timeout,
            bootstrap_from_snapshot = self.bootstrap_from_snapshot,
            client_tls_config = self.client_tls_config,
            server_tls_config = self.server_tls_config,
        )
    }
}
