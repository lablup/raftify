use raftify::{InitialRole, Peers};
use serde::Deserialize;
use slog::{o, Drain};
use slog_envlogger::LogBuilder;
use std::net::SocketAddr;
use std::path::Path;
use std::process::Command;
use std::str;
use std::str::FromStr;
use std::{fs, time::Duration};
use tokio::time::sleep;
use toml;

use crate::constant::RAFT_PORTS;
use crate::{constant::ZERO_NODE_EXAMPLE, raft::Raft};

pub fn build_logger() -> slog::Logger {
    let decorator = slog_term::TermDecorator::new().build();
    let drain = slog_term::CompactFormat::new(decorator).build();
    let drain = std::sync::Mutex::new(drain).fuse();

    let mut builder = LogBuilder::new(drain);
    builder = builder.filter(None, slog::FilterLevel::Debug);

    if let Ok(s) = std::env::var("RUST_LOG") {
        builder = builder.parse(&s);
    }
    let drain = builder.build();

    slog::Logger::root(drain, o!())
}

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

pub async fn load_peers(example_filename: &str) -> Result<Peers, Box<dyn std::error::Error>> {
    if example_filename == ZERO_NODE_EXAMPLE {
        return Ok(Peers::with_empty());
    }

    let path = Path::new("fixtures").join(example_filename);
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

pub async fn wait_for_until_cluster_size_increase(raft: Raft, target: usize) {
    raft.logger.debug(&format!(
        "Waiting for cluster size to increase to... {}",
        target
    ));

    loop {
        let size = raft.raft_node.get_cluster_size().await;
        if size >= target {
            break;
        }
        sleep(Duration::from_secs_f32(0.1)).await;
    }

    // Wait for the conf_change reflected to the cluster
    sleep(Duration::from_secs_f32(1.0)).await;
}

pub async fn wait_for_until_cluster_size_decrease(raft: Raft, target: usize) {
    raft.logger.debug(&format!(
        "Waiting for cluster size to decrease to {}...",
        target
    ));

    loop {
        let size = raft.raft_node.get_cluster_size().await;
        if size <= target {
            break;
        }
        sleep(Duration::from_secs_f32(0.1)).await;
    }

    // Wait for the conf_change reflected to the cluster
    sleep(Duration::from_secs_f32(1.0)).await;
}

pub fn kill_process_using_port(port: u16) {
    let port_str = port.to_string();

    let output = Command::new("lsof")
        .arg("-i")
        .arg(format!(":{}", port_str))
        .arg("-t")
        .output()
        .expect("Failed to execute lsof command");

    let pid = str::from_utf8(&output.stdout).unwrap().trim();

    if pid.is_empty() {
        return;
    }

    Command::new("kill")
        .arg("-9")
        .arg(pid)
        .output()
        .expect("Failed to execute kill command");
}

pub fn kill_previous_raft_processes() {
    RAFT_PORTS.iter().for_each(|port| {
        kill_process_using_port(*port);
    });
}
