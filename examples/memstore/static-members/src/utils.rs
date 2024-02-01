use raftify::{InitialRole, Peers};
use serde::Deserialize;
use std::fs;
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

pub async fn load_peers() -> Result<Peers, Box<dyn std::error::Error>> {
    let path = Path::new(file!())
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("cluster_config.toml");
    let config_str = fs::read_to_string(path)?;

    let raft_config: TomlRaftConfig = toml::from_str(&config_str)?;

    let mut peers = Peers::with_empty();

    for peer_info in raft_config.raft.peers {
        let addr = SocketAddr::new(peer_info.ip.parse().unwrap(), peer_info.port);
        let role = InitialRole::from_str(&peer_info.role).expect("Invalid role!");
        peers.add_peer(peer_info.node_id, addr, Some(role));
    }

    Ok(peers)
}
