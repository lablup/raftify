use config::{Config as BaseConfig, ConfigError, File};
use raft::Config as RaftConfig;
use serde::{Deserialize, Serialize};

use crate::{error::Error, error::Result, peers::Peers, InitialRole};
pub mod config_builder;

// NOTE: TlsConfig is used in both RaftServer and RaftClient.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct TlsConfig {
    pub cert_path: Option<String>,
    pub key_path: Option<String>,
    pub ca_cert_path: Option<String>,
    pub domain_name: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
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

pub fn load_configs(filename: &str) -> std::result::Result<Config, ConfigError> {
    let file_config_builder = BaseConfig::builder().add_source(File::with_name(filename));
    let file_config = file_config_builder.build()?;

    file_config.try_deserialize()
}
