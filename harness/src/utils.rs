use raftify::raft::StateRole;
use raftify::{Error, InitialRole, Peers, StableStorage};
use serde::Deserialize;
use slog::{o, Drain};
use slog_envlogger::LogBuilder;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::ErrorKind;
use std::net::SocketAddr;
use std::path::Path;
use std::process::Command;
use std::str;
use std::str::FromStr;
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

pub fn kill_process_using_port(port: u16) {
    let port_str = port.to_string();
    #[cfg(target_os = "windows")]
    {
        let output = Command::new("netstat")
            .arg("-ano")
            .output()
            .expect("Failed to execute netstat command");

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut pid = String::new();

        for line in stdout.lines() {
            if line.contains(&port_str) {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if let Some(pid_part) = parts.last() {
                    pid = pid_part.to_string();
                    break;
                }
            }
        }

        if pid.is_empty() {
            return;
        }

        Command::new("taskkill")
            .arg("/PID")
            .arg(&pid)
            .arg("/F")
            .output()
            .expect("Failed to execute taskkill command");
    }

    #[cfg(not(target_os = "windows"))]
    {
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
}

pub fn cleanup_storage(log_dir: &str) {
    let storage_pth = Path::new(log_dir);

    if fs::metadata(storage_pth).is_ok() {
        fs::remove_dir_all(storage_pth).expect("Failed to remove storage directory");
    }

    fs::create_dir_all(storage_pth).expect("Failed to create storage directory");
}

/// Collects the IDs of Raft nodes that match the target role during the latest term.
/// This function continuously checks the nodes until a Leader is found or the term changes.
///
/// # Note
///
/// It may potentially hang if a leader does not emerge due to lack of quorum.
///
/// # Examples
/// ```rust, ignore
/// let rafts = wait_until_rafts_ready(None, rx_raft, 5).await;
/// let leader_id = rafts.get(&1).unwrap().get_leader_id().await;
/// let leader_set =
///     collect_state_matching_rafts_until_leader_emerge(&rafts, StateRole::Leader).await;
/// let follower_set =
///     collect_state_matching_rafts_until_leader_emerge(&rafts, StateRole::Follower).await;
///
/// assert_eq!(1, leader_set.len());
/// assert!(leader_set.get(&leader_id).is_some());
/// assert_eq!(4, follower_set.len());
/// assert!(follower_set.get(&leader_id).is_none());
/// ```
pub async fn collect_state_matching_rafts_until_leader_emerge(
    rafts: &HashMap<u64, Raft>,
    target_role: StateRole,
) -> HashSet<u64> {
    let mut result = HashSet::with_capacity(rafts.len());
    let mut current_term = 1;
    let mut leader_emerged = false;
    while !leader_emerged {
        for (id, raft) in rafts.iter() {
            let (term, state_role) = (
                raft.raft_node
                    .storage()
                    .await
                    .unwrap()
                    .hard_state()
                    .unwrap()
                    .get_term(),
                unsafe { raft.get_raw_node().await.unwrap().lock().await.raft.state },
            );

            if current_term > term {
                continue;
            }
            if current_term < term {
                result.clear();
                current_term = term;
            }
            if state_role == target_role {
                result.insert(*id);
            }
            if state_role == StateRole::Leader {
                leader_emerged = true;
            }
        }
    }
    result
}

pub fn kill_previous_raft_processes() {
    RAFT_PORTS.iter().for_each(|port| {
        kill_process_using_port(*port);
    });
}

pub fn get_storage_path(log_dir: &str, node_id: u64) -> String {
    format!("{}/node-{}", log_dir, node_id)
}

pub fn get_data_mdb_path(log_dir: &str, node_id: u64) -> String {
    format!("{}/data.mdb", get_storage_path(log_dir, node_id))
}

pub fn ensure_directory_exist(dir_pth: &str) -> Result<(), Error> {
    let dir_pth: &Path = Path::new(&dir_pth);

    if fs::metadata(dir_pth).is_err() {
        fs::create_dir_all(dir_pth)?;
    }
    Ok(())
}

#[tokio::test]
async fn test_collect_candidate_rafts_until_leader_emerge() {
    use crate::constant::FIVE_NODE_EXAMPLE;
    use crate::raft::{build_raft_cluster, wait_until_rafts_ready};
    use std::sync::mpsc;

    cleanup_storage("./logs");
    kill_previous_raft_processes();
    let (tx_raft, rx_raft) = mpsc::channel::<(u64, Raft)>();
    let peers = load_peers(FIVE_NODE_EXAMPLE).await.unwrap();
    let _raft_tasks = tokio::spawn(build_raft_cluster(tx_raft, peers.clone()));

    let rafts = wait_until_rafts_ready(None, rx_raft, 5).await;
    let leader_id = rafts.get(&1).unwrap().get_leader_id().await.unwrap();
    let leader_set =
        collect_state_matching_rafts_until_leader_emerge(&rafts, StateRole::Leader).await;
    let follower_set =
        collect_state_matching_rafts_until_leader_emerge(&rafts, StateRole::Follower).await;

    assert_eq!(1, leader_set.len());
    assert!(leader_set.get(&leader_id).is_some());
    assert_eq!(4, follower_set.len());
    assert!(follower_set.get(&leader_id).is_none());

    assert!(
        collect_state_matching_rafts_until_leader_emerge(&rafts, StateRole::Candidate)
            .await
            .is_empty()
    );
    assert!(
        collect_state_matching_rafts_until_leader_emerge(&rafts, StateRole::PreCandidate)
            .await
            .is_empty()
    );
    std::process::exit(0);
}
