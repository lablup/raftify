use serde::Deserialize;
use std::net::SocketAddr;
use std::path::Path;
use std::{fs, time::Duration};
use tokio::time::sleep;
use toml;

use raftify::{Config, Peers, RaftConfig};

use crate::raft_server::Raft;

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
    let path = Path::new("fixtures").join(filename);
    let config_str = fs::read_to_string(path)?;

    let raft_config: TomlRaftConfig = toml::from_str(&config_str)?;

    let mut peers = Peers::new();

    for peer_info in raft_config.raft.peers {
        let addr = SocketAddr::new(peer_info.ip.parse().unwrap(), peer_info.port);
        peers.add_peer(peer_info.node_id, addr);
    }

    Ok(peers)
}

pub async fn wait_for_until_cluster_size_increase(raft: Raft, target: usize) {
    println!("Waiting for cluster size to increase to {}...", target);

    loop {
        let size = raft.cluster_size().await;
        if size >= target {
            break;
        }
        sleep(Duration::from_secs_f32(0.1)).await;
    }

    // Wait for the conf_change reflected to the cluster
    sleep(Duration::from_secs_f32(0.1)).await;
}

pub async fn wait_for_until_cluster_size_decrease(raft: Raft, target: usize) {
    println!("Waiting for cluster size to decrease to {}...", target);

    loop {
        let size = raft.cluster_size().await;
        if size <= target {
            break;
        }
        sleep(Duration::from_secs_f32(0.1)).await;
    }

    // Wait for the conf_change reflected to the cluster
    sleep(Duration::from_secs_f32(0.1)).await;
}
