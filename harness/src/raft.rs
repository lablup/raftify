use futures::future;
use raftify::{
    raft::{formatter::set_custom_formatter, logger::Slogger},
    CustomFormatter, HeedStorage, Peers, Raft as Raft_, Result, StableStorage,
};
use std::{
    collections::HashMap,
    sync::{mpsc, Arc},
    time::Duration,
};
use tokio::task::JoinHandle;

use crate::{
    config::build_config,
    logger::get_logger,
    state_machine::{HashStore, LogEntry},
    utils::{build_logger, ensure_directory_exist, get_storage_path},
};

pub type Raft = Raft_<LogEntry, HeedStorage, HashStore>;

pub async fn wait_until_rafts_ready(
    rafts: Option<HashMap<u64, Raft>>,
    rx_initialized_raft: mpsc::Receiver<(u64, Raft)>,
    size: usize,
) -> HashMap<u64, Raft> {
    let logger = get_logger();
    let mut rafts = rafts.unwrap_or_default();

    loop {
        slog::info!(logger, "Waiting for all raft instances to be ready...");
        tokio::task::yield_now().await;

        let (node_id, raft) = rx_initialized_raft
            .recv()
            .expect("All tx dropped before receiving all raft instances");
        let term = raft
            .raft_node
            .storage()
            .await
            .unwrap()
            .hard_state()
            .unwrap()
            .get_term();
        if term >= 1 {
            rafts.insert(node_id, raft);
        }
        if rafts.len() >= size {
            break;
        }
        tokio::time::sleep(Duration::from_secs_f32(0.5)).await;
    }
    rafts
}

fn run_raft(
    tx_initialized_raft: mpsc::Sender<(u64, Raft)>,
    node_id: &u64,
    peers: Peers,
    should_be_leader: bool,
) -> Result<JoinHandle<Result<()>>> {
    let peer = peers.get(node_id).unwrap();
    let cfg = build_config(
        *node_id,
        if should_be_leader {
            None
        } else {
            Some(peers.clone())
        },
    );

    let store = HashStore::new();
    let logger = build_logger();
    let storage_pth = get_storage_path(cfg.get_log_dir(), *node_id);
    ensure_directory_exist(storage_pth.as_str())?;

    let storage = HeedStorage::create(
        &storage_pth,
        &cfg,
        Arc::new(Slogger {
            slog: logger.clone(),
        }),
    )?;

    let raft = Raft::bootstrap(
        *node_id,
        peer.addr,
        storage,
        store,
        cfg,
        Arc::new(Slogger {
            slog: logger.clone(),
        }),
    )
    .expect("Raft build failed!");

    tx_initialized_raft
        .send((node_id.to_owned(), raft.clone()))
        .expect("Failed to send raft to the channel");

    let raft_handle = tokio::spawn(raft.clone().run());

    Ok(raft_handle)
}

pub async fn build_raft_cluster(
    tx_initialized_raft: mpsc::Sender<(u64, Raft)>,
    peers: Peers,
) -> Result<()> {
    let logger = get_logger();

    set_custom_formatter(CustomFormatter::<LogEntry, HashStore>::new());

    let mut raft_handles = vec![];
    let should_be_leader = peers.len() <= 1;

    for (node_id, _) in peers.iter() {
        let raft_handle = run_raft(
            tx_initialized_raft.clone(),
            &node_id,
            peers.clone(),
            should_be_leader,
        )?;
        raft_handles.push(raft_handle);

        slog::info!(logger, "Node {} starting...", node_id);
    }

    let results = future::join_all(raft_handles).await;

    for (result_idx, result) in results.iter().enumerate() {
        match result {
            Ok(_) => slog::info!(logger, "All tasks completed successfully"),
            Err(e) => slog::error!(
                logger,
                "Error occurred while running node {}. Error: {:?}",
                result_idx + 1,
                e
            ),
        }
    }

    Ok(())
}

pub async fn spawn_extra_node(
    tx_initialized_raft: mpsc::Sender<(u64, Raft)>,
    node_id: u64,
    raft_addr: &str,
) -> Result<JoinHandle<Result<()>>> {
    let logger = Arc::new(Slogger {
        slog: build_logger(),
    });

    let cfg = build_config(node_id, None);
    let store = HashStore::new();
    let storage_pth = get_storage_path(cfg.get_log_dir(), node_id);
    ensure_directory_exist(storage_pth.as_str())?;

    let storage = HeedStorage::create(&storage_pth, &cfg, logger.clone())?;
    let raft = Raft::bootstrap(node_id, raft_addr, storage, store, cfg, logger)
        .expect("Raft build failed!");

    tx_initialized_raft
        .send((node_id, raft.clone()))
        .expect("Failed to send raft to the channel");

    let raft_handle = tokio::spawn(raft.clone().run());

    Ok(raft_handle)
}

pub async fn spawn_and_join_extra_node(
    tx_initialized_raft: mpsc::Sender<(u64, Raft)>,
    raft_addr: &str,
    peer_addr: &str,
) -> Result<JoinHandle<Result<()>>> {
    let logger = Arc::new(Slogger {
        slog: build_logger(),
    });
    let join_ticket = Raft::request_id(raft_addr.to_owned(), peer_addr.to_owned(), None)
        .await
        .unwrap();

    let node_id = join_ticket.reserved_id;
    let cfg = build_config(node_id, Some(join_ticket.peers.clone().into()));

    let store = HashStore::new();

    let storage_pth = get_storage_path(cfg.get_log_dir(), node_id);
    ensure_directory_exist(storage_pth.as_str())?;

    let storage = HeedStorage::create(&storage_pth, &cfg, logger.clone())?;
    let raft = Raft::bootstrap(node_id, raft_addr, storage, store, cfg, logger)
        .expect("Raft build failed!");

    tx_initialized_raft
        .send((node_id, raft.clone()))
        .expect("Failed to send raft to the channel");

    let raft_handle = tokio::spawn(raft.clone().run());

    raft.add_peers(join_ticket.peers.clone())
        .await
        .expect("Failed to add peers");
    raft.join_cluster(vec![join_ticket])
        .await
        .expect("Failed to join cluster");

    Ok(raft_handle)
}

pub async fn join_nodes(rafts: Vec<&Raft>, raft_addrs: Vec<&str>, peer_addr: &str) {
    let mut tickets = vec![];
    for (raft, raft_addr) in rafts.iter().zip(raft_addrs.into_iter()) {
        let join_ticket = Raft::request_id(raft_addr.to_owned(), peer_addr.to_owned(), None)
            .await
            .unwrap();

        raft.add_peers(join_ticket.peers.clone())
            .await
            .expect("Failed to add peers");
        tickets.push(join_ticket);
    }

    rafts[0]
        .join_cluster(tickets)
        .await
        .expect("Failed to join cluster");
}
