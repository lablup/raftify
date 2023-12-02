use serde::Deserialize;
use std::fs;
use std::net::SocketAddr;
use std::path::Path;
use toml;

use riteraft::{Config, Peers, RaftConfig};

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

#[derive(Deserialize)]
pub struct RaftPeer {
    pub ip: String,
    pub port: u16,
    pub node_id: u64,
}

#[derive(Deserialize)]
pub struct RaftConfiguration {
    peers: Vec<RaftPeer>,
}

#[derive(Deserialize)]
pub struct Configuration {
    cfg: RaftConfiguration,
}

pub async fn load_peers() -> Result<Peers, Box<dyn std::error::Error>> {
    let path = Path::new(file!()).parent().unwrap().join("config.toml");
    let config_str = fs::read_to_string(path)?;

    let raft_config: Configuration = toml::from_str(&config_str)?;

    let mut peers = Peers::new();

    for peer_info in raft_config.cfg.peers {
        let addr = SocketAddr::new(peer_info.ip.parse().unwrap(), peer_info.port);
        peers.add_peer(peer_info.node_id, addr).await;
    }

    Ok(peers)
}
