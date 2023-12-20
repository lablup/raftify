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

#[derive(Deserialize, Debug)]
pub struct TomlRaftPeer {
    pub ip: String,
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

pub async fn load_peers(filename: &str) -> Result<Peers, Box<dyn std::error::Error>> {
    let path = Path::new(file!())
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join(filename);
    let config_str = fs::read_to_string(path)?;

    let raft_config: TomlRaftConfig = toml::from_str(&config_str)?;

    let mut peers = Peers::new();

    for peer_info in raft_config.raft.peers {
        let addr = SocketAddr::new(peer_info.ip.parse().unwrap(), peer_info.port);
        peers.add_peer(peer_info.node_id, addr);
    }

    Ok(peers)
}
