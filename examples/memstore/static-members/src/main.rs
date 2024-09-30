#[macro_use]
extern crate slog;
extern crate slog_async;
extern crate slog_term;

use actix_web::{web, App, HttpServer};
use raftify::{
    raft::{formatter::set_custom_formatter, logger::Slogger},
    CustomFormatter,
};
use slog::Drain;
use slog_envlogger::LogBuilder;
use std::sync::Arc;
use structopt::StructOpt;

use example_harness::{
    config::build_config,
    utils::{ensure_directory_exist, get_storage_path},
};
use memstore_example_harness::{
    state_machine::{HashStore, LogEntry, Raft},
    web_server_api::{
        campaign, debug, demote, get, leader_id, leave, leave_joint, peers, put, snapshot,
        transfer_leader,
    },
};
use memstore_static_members::utils::load_peers;

#[cfg(feature = "inmemory_storage")]
use raftify::MemStorage;

#[cfg(feature = "heed_storage")]
use raftify::HeedStorage;

#[cfg(feature = "rocksdb_storage")]
use raftify::RocksDBStorage;

#[derive(Debug, StructOpt)]
struct Options {
    #[structopt(long)]
    raft_addr: String,
    #[structopt(long)]
    web_server: Option<String>,
    // TODO: Make "bootstrap_from_snapshot" option here
}

#[actix_rt::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    color_backtrace::install();

    let decorator = slog_term::TermDecorator::new().build();
    let drain = slog_term::FullFormat::new(decorator).build().fuse();
    let drain = slog_async::Async::new(drain).build().fuse();

    let mut builder = LogBuilder::new(drain);
    builder = builder.filter(None, slog::FilterLevel::Debug);

    if let Ok(s) = std::env::var("RUST_LOG") {
        builder = builder.parse(&s);
    }
    let drain = builder.build().fuse();

    let logger = Arc::new(Slogger {
        slog: slog::Logger::root(drain, o!()),
    });

    set_custom_formatter(CustomFormatter::<LogEntry, HashStore>::new());

    let options = Options::from_args();
    let store = HashStore::new();
    let initial_peers = load_peers("cluster_config.toml").await?;

    let node_id = initial_peers
        .get_node_id_by_addr(options.raft_addr.clone())
        .unwrap();

    let mut cfg = build_config(node_id);
    cfg.initial_peers = Some(initial_peers.clone());

    let storage_pth = get_storage_path(cfg.log_dir.as_str(), node_id);
    ensure_directory_exist(storage_pth.as_str())?;

    #[cfg(feature = "inmemory_storage")]
    let log_storage = MemStorage::create();

    #[cfg(feature = "heed_storage")]
    let log_storage = HeedStorage::create(&storage_pth, &cfg.clone(), logger.clone())
        .expect("Failed to create heed storage");

    #[cfg(feature = "rocksdb_storage")]
    let log_storage = RocksDBStorage::create(&storage_pth, logger.clone())
        .expect("Failed to create heed storage");

    let raft = Raft::bootstrap(
        node_id,
        options.raft_addr.clone(),
        log_storage,
        store.clone(),
        cfg.clone(),
        logger.clone(),
    )?;

    let handle = tokio::spawn(raft.clone().run());

    if let Some(addr) = options.web_server {
        let _web_server = tokio::spawn(
            HttpServer::new(move || {
                App::new()
                    .app_data(web::Data::new((store.clone(), raft.clone())))
                    .service(put)
                    .service(get)
                    .service(leave)
                    .service(debug)
                    .service(peers)
                    .service(snapshot)
                    .service(leader_id)
                    .service(leave_joint)
                    .service(transfer_leader)
                    .service(campaign)
                    .service(demote)
            })
            .bind(addr)
            .unwrap()
            .run(),
        );
    }

    let result = tokio::try_join!(handle)?;
    result.0?;
    Ok(())
}
