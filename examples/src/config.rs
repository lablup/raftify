use raftify::{Config, ConfigBuilder, Peers, RaftConfig};

#[cfg(feature = "tls")]
use raftify::TlsConfig;

use crate::utils::{ensure_directory_exist, get_storage_path};

pub fn build_config(node_id: u64, initial_peers: Option<Peers>) -> Config {
    let raft_config = RaftConfig {
        id: node_id,
        election_tick: 10,
        heartbeat_tick: 3,
        omit_heartbeat_log: false,
        ..Default::default()
    };

    let storage_pth = get_storage_path("./logs", node_id);
    ensure_directory_exist(&storage_pth).expect("Failed to create storage directory");

    #[allow(unused_mut)]
    let mut config_builder = ConfigBuilder::new()
        .log_dir("./logs".to_owned())
        .save_compacted_logs(true)
        .compacted_log_dir("./logs".to_owned())
        .compacted_log_size_threshold(1024 * 1024 * 1024)
        .raft_config(raft_config)
        .tick_interval(0.2);

    #[cfg(feature = "tls")]
    {
        let client_tls_config = TlsConfig {
            cert_path: None,
            key_path: None,
            ca_cert_path: Some("./ca_cert.pem".to_string()),
            domain_name: Some("localhost".to_string()),
        };

        config_builder = config_builder.global_client_tls_config(client_tls_config);
    }

    let config_builder = if let Some(peers) = initial_peers {
        config_builder.initial_peers(peers)
    } else {
        config_builder
    };

    config_builder.build()
}
