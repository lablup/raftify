use futures::future;
use once_cell::sync::Lazy;
use raftify::{
    raft::{formatter::set_custom_formatter, logger::Slogger},
    CustomFormatter, Peers, Raft as Raft_, Result,
};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use tokio::task::JoinHandle;

use crate::{
    config::build_config,
    state_machine::{HashStore, LogEntry},
    utils::build_logger,
};

pub type Raft = Raft_<LogEntry, HashStore>;

pub static RAFTS: Lazy<Mutex<HashMap<u64, Raft>>> = Lazy::new(|| Mutex::new(HashMap::new()));

fn run_raft(node_id: &u64, peers: Peers, should_be_leader: bool) -> Result<JoinHandle<Result<()>>> {
    let peer = peers.get(node_id).unwrap();
    let mut cfg = build_config();
    cfg.initial_peers = if should_be_leader {
        None
    } else {
        Some(peers.clone())
    };

    let store = HashStore::new();
    let logger = build_logger();

    let raft = Raft::bootstrap(
        *node_id,
        peer.addr,
        store,
        cfg,
        Arc::new(Slogger {
            slog: logger.clone(),
        }),
    )
    .expect("Raft build failed!");

    RAFTS.lock().unwrap().insert(*node_id, raft.clone());

    let raft_handle = tokio::spawn(raft.clone().run());

    Ok(raft_handle)
}

pub async fn build_raft_cluster(peers: Peers) -> Result<()> {
    set_custom_formatter(CustomFormatter::<LogEntry, HashStore>::new());

    let mut raft_handles = vec![];
    let should_be_leader = peers.len() <= 1;

    for (node_id, _) in peers.iter() {
        let raft_handle = run_raft(&node_id, peers.clone(), should_be_leader)?;
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

// Note that this function lock RAFTS, so it should not be called while holding RAFTS lock.
pub async fn spawn_extra_node(raft_addr: &str, peer_addr: &str) -> Result<JoinHandle<Result<()>>> {
    let logger = Arc::new(Slogger {
        slog: build_logger(),
    });
    let join_ticket = Raft::request_id(raft_addr.to_owned(), peer_addr.to_owned())
        .await
        .unwrap();

    let node_id = join_ticket.reserved_id;
    let mut cfg = build_config();
    cfg.initial_peers = Some(join_ticket.peers.clone().into());
    let store = HashStore::new();

    let raft = Raft::bootstrap(node_id, raft_addr, store, cfg, logger).expect("Raft build failed!");

    RAFTS.lock().unwrap().insert(node_id, raft.clone());

    let raft_handle = tokio::spawn(raft.clone().run());

    raft.raft_node.add_peers(join_ticket.peers.clone()).await;
    raft.join(join_ticket).await;

    Ok(raft_handle)
}
