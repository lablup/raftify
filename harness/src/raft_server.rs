use futures::future;
use once_cell::sync::Lazy;
use raftify::{
    raft::{
        formatter::set_custom_formatter,
        logger::{Logger, Slogger},
    },
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

fn run_raft(node_id: &u64, peers: Peers) -> Result<JoinHandle<Result<()>>> {
    let peer = peers.get(node_id).unwrap();
    let cfg = build_config();
    let store = HashStore::new();
    let logger = build_logger();

    let raft = Raft::build(
        *node_id,
        peer.addr,
        store,
        cfg,
        Arc::new(Slogger {
            slog: logger.clone(),
        }),
        Some(peers.clone()),
    )
    .expect("Raft build failed!");

    RAFTS.lock().unwrap().insert(*node_id, raft.clone());

    let raft_handle = tokio::spawn(raft.clone().run());

    Ok(raft_handle)
}

pub async fn run_rafts(peers: Peers) -> Result<()> {
    set_custom_formatter(CustomFormatter::<LogEntry, HashStore>::new());

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

pub async fn handle_bootstrap(peers: Peers, logger: Arc<dyn Logger>) -> Result<()> {
    let leader_addr = peers.get(&1).unwrap().addr;

    for (node_id, _) in peers.iter() {
        if node_id != 1 {
            Raft::member_bootstrap_ready(leader_addr, node_id, logger.clone()).await?;
        }
    }

    Ok(())
}

pub async fn spawn_extra_node(peer_addr: &str, raft_addr: &str) -> Result<JoinHandle<Result<()>>> {
    let logger = Arc::new(Slogger {
        slog: build_logger(),
    });
    let join_ticket = Raft::request_id(raft_addr.to_owned(), peer_addr.to_owned(), logger.clone())
        .await
        .unwrap();

    let node_id = join_ticket.reserved_id;
    let cfg = build_config();
    let store = HashStore::new();

    let raft =
        Raft::build(node_id, raft_addr, store, cfg, logger, None).expect("Raft build failed!");

    RAFTS.lock().unwrap().insert(node_id, raft.clone());

    let raft_handle = tokio::spawn(raft.clone().run());

    raft.join(join_ticket).await;

    Ok(raft_handle)
}
