use std::path::Path;

use raftify::{load_configs, Config, ConfigBuilder, Peers};

#[cfg(feature = "tls")]
use raftify::TlsConfig;

use crate::utils::{ensure_directory_exist, get_storage_path};

pub fn build_config(node_id: u64, initial_peers: Option<Peers>) -> Config {
    let storage_pth = get_storage_path("./logs", node_id);
    ensure_directory_exist(&storage_pth).expect("Failed to create storage directory");

    let path = Path::new(file!())
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("memstore/common_raftnode_config.toml");

    #[allow(unused_mut)]
    let mut config_builder = ConfigBuilder::from_config(
        load_configs(path.to_str().unwrap()).expect("Failed to load common config"),
    )
    .log_dir(storage_pth.clone())
    .compacted_log_dir(storage_pth)
    .set_node_id(node_id);

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
