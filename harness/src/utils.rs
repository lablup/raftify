use raftify::raft::StateRole;
use raftify::{Error, InitialRole, Peers, StableStorage};
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::ErrorKind;
use std::net::SocketAddr;
use std::path::Path;
use std::str;
use std::str::FromStr;
use toml;

use crate::{constant::ZERO_NODE_EXAMPLE, raft::Raft};

#[derive(Deserialize, Debug)]
pub struct TomlRaftPeer {
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
    loopback_address: &str,
    example_filename: &str,
) -> Result<Peers, Box<dyn std::error::Error>> {
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
        let addr = SocketAddr::new(loopback_address.parse().unwrap(), peer_info.port);
        let role = InitialRole::from_str(&peer_info.role).expect("Invalid role!");
        peers.add_peer(peer_info.node_id, addr, Some(role));
    }

    Ok(peers)
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

/// Collects the IDs and roles of all Raft nodes during the latest term.
/// This function continuously checks the nodes until a leader is elected or the term changes.
/// It then returns a set containing each node's ID and its current role.
///
/// # Note
///
/// It may potentially hang if a leader does not emerge due to lack of quorum.
/// ```
pub async fn gather_rafts_when_leader_elected(
    rafts: &HashMap<u64, Raft>,
) -> HashMap<StateRole, HashSet<u64>> {
    let mut result: HashMap<StateRole, HashSet<u64>> = HashMap::from([
        (StateRole::Follower, HashSet::new()),
        (StateRole::Candidate, HashSet::new()),
        (StateRole::Leader, HashSet::new()),
        (StateRole::PreCandidate, HashSet::new()),
    ]);

    let mut current_term = 1;
    let mut leader_elected = false;

    while !leader_elected {
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
                result.values_mut().for_each(|v| v.clear());
                current_term = term;
            }

            result.get_mut(&state_role).unwrap().insert(*id);

            if state_role == StateRole::Leader {
                leader_elected = true;
            }
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use raftify::{raft::StateRole, HeedStorage, Raft as Raft_};

    use crate::{
        state_machine::{HashStore, LogEntry},
        test_environment::prepare_test_environment,
        utils::{gather_rafts_when_leader_elected, load_peers},
    };

    pub type Raft = Raft_<LogEntry, HeedStorage, HashStore>;

    #[tokio::test]
    async fn test_gather_rafts_when_leader_elected() {
        use crate::constant::FIVE_NODE_EXAMPLE;
        use crate::raft::{build_raft_cluster, wait_until_rafts_ready};
        use std::sync::mpsc;

        let test_environment =
            prepare_test_environment(stringify!(test_gather_rafts_when_leader_elected));

        let (tx_raft, rx_raft) = mpsc::channel::<(u64, Raft)>();
        let peers = load_peers(&test_environment.loopback_address, FIVE_NODE_EXAMPLE)
            .await
            .unwrap();
        let _raft_tasks = tokio::spawn(build_raft_cluster(
            tx_raft,
            test_environment.base_storage_path,
            peers.clone(),
        ));

        let rafts = wait_until_rafts_ready(None, rx_raft, 5).await;

        let all_rafts = gather_rafts_when_leader_elected(&rafts).await;

        let leaders = all_rafts.get(&StateRole::Leader).unwrap();
        let followers = all_rafts.get(&StateRole::Follower).unwrap();
        let candidates = all_rafts.get(&StateRole::Candidate).unwrap();
        let precandidates = all_rafts.get(&StateRole::PreCandidate).unwrap();

        assert_eq!(1, leaders.len());
        assert_eq!(4, followers.len());
        assert_eq!(0, candidates.len());
        assert_eq!(0, precandidates.len());

        std::process::exit(0);
    }
}
