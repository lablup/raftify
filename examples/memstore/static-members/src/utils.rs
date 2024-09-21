use raftify::{InitialRole, Peers};
use serde::Deserialize;
use std::fs;
use std::io::ErrorKind;
use std::net::SocketAddr;
use std::path::Path;
use std::str::FromStr;
use toml;

#[derive(Deserialize, Debug)]
pub struct TomlRaftPeer {
    pub ip: String,
    pub role: String,
    pub port: u16,
    pub node_id: u64,
}

#[derive(Deserialize, Debug)]
pub struct TomlRaftConfig {
    pub raft: TomlInnerRaftConfig,
}

#[derive(Deserialize, Debug)]
pub struct TomlInnerRaftConfig {
    pub peers: Vec<TomlRaftPeer>,
}

pub async fn load_peers(
    cluster_config_filename: &str,
) -> Result<Peers, Box<dyn std::error::Error>> {
    let path = Path::new(file!())
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join(cluster_config_filename);

    let config_str = match fs::read_to_string(&path) {
        Ok(content) => content,
        Err(e) => {
            if e.kind() == ErrorKind::NotFound {
                let path = path.into_os_string().into_string().unwrap();
                return Err(
                    format!("Cluster configuration file not found at path: {}", path).into(),
                );
            } else {
                return Err(e.into());
            }
        }
    };

    let raft_config: TomlRaftConfig = toml::from_str(&config_str)?;

    let mut peers = Peers::with_empty();

    for peer_info in raft_config.raft.peers {
        let addr = SocketAddr::new(peer_info.ip.parse().unwrap(), peer_info.port);
        let role = InitialRole::from_str(&peer_info.role).expect("Invalid role!");
        peers.add_peer(peer_info.node_id, addr, Some(role));
    }

    Ok(peers)
}
