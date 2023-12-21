use futures::future;
use once_cell::sync::Lazy;
use raftify::{raft::default_logger, Peers, Raft as Raft_, Result};
use std::sync::Mutex;
use tokio::task::JoinHandle;

use crate::{
    state_machine::{HashStore, LogEntry},
    utils::build_config,
};

pub type Raft = Raft_<LogEntry, HashStore>;

pub static RAFTS: Lazy<Mutex<Vec<Raft>>> = Lazy::new(|| Mutex::new(vec![]));

fn run_raft(node_id: &u64, peers: Peers) -> Result<JoinHandle<Result<()>>> {
    let peer = peers.get(node_id).unwrap();
    let cfg = build_config();
    let store = HashStore::new();
    let logger = default_logger();

    let raft = Raft::build(
        *node_id,
        peer.addr,
        store,
        cfg,
        logger.clone(),
        Some(peers.clone()),
    )
    .expect("Raft build failed!");

    RAFTS.lock().unwrap().push(raft.clone());

    let raft_handle = tokio::spawn(raft.clone().run());

    Ok(raft_handle)
}

pub async fn run_rafts(peers: Peers) -> Result<()> {
    let mut raft_handles = vec![];

    for (node_id, _) in peers.iter() {
        let raft_handle = run_raft(&node_id, peers.clone())?;
        raft_handles.push(raft_handle);
        println!("Node {} starting...", node_id);
    }

    let results = future::join_all(raft_handles).await;

    for (result_idx, result) in results.iter().enumerate() {
        match result {
            Ok(_) => println!("All tasks completed successfully"),
            Err(e) => println!(
                "Error occurred while running node {}. Error: {:?}",
                result_idx + 1,
                e
            ),
        }
    }

    Ok(())
}

pub async fn bootstrap(peers: Peers) -> Result<()> {
    let leader_addr = peers.get(&1).unwrap().addr;

    for (node_id, _) in peers.iter() {
        if node_id != 1 {
            let mut raft_lk = RAFTS.lock().unwrap();
            let raft = raft_lk.get_mut(node_id as usize - 1).unwrap();
            raft.member_bootstrap_ready(leader_addr, node_id).await?;
        }
    }

    Ok(())
}
