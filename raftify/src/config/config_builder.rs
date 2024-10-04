use raft::Config as RaftConfig;
use serde::{Deserialize, Serialize};

use crate::Peers;

use super::{Config, TlsConfig};

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ConfigBuilder {
    pub(crate) config: Config,
    global_client_tls_config: Option<TlsConfig>,
}

impl ConfigBuilder {
    pub fn new() -> Self {
        Self {
            config: Config::default(),
            global_client_tls_config: None,
        }
    }

    pub fn from_config(config: Config) -> Self {
        Self {
            config,
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

    pub fn set_node_id(mut self, node_id: u64) -> Self {
        self.config.raft_config.id = node_id;
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
                } else {
                    self.config.client_tls_config = self_peer.client_tls_config.clone();
                }
            }
            self.config.initial_peers = Some(initial_peers);
        }

        self.config
    }
}
