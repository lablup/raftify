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
use std::{fs, path::Path, sync::Arc};
use structopt::StructOpt;

use example_harness::config::build_config;
use memstore_example_harness::{
    state_machine::{HashStore, LogEntry, Raft},
    web_server_api::{
        campaign, debug, demote, get, leader, leave, leave_joint, peers, put, snapshot,
        transfer_leader,
    },
};

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
    peer_addr: Option<String>,
    #[structopt(long)]
    web_server: Option<String>,
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

    let (raft, raft_handle) = match options.peer_addr {
        Some(peer_addr) => {
            log::info!("Running in Follower mode");

            let ticket = Raft::request_id(options.raft_addr.clone(), peer_addr.clone(), None)
                .await
                .unwrap();
            let node_id = ticket.reserved_id;
            let cfg = build_config(node_id, Some(ticket.peers.clone().into()));

            #[cfg(feature = "inmemory_storage")]
            let log_storage = MemStorage::create();

            #[cfg(feature = "heed_storage")]
            let log_storage = HeedStorage::create(cfg.get_log_dir(), &cfg.clone(), logger.clone())
                .expect("Failed to create storage");

            #[cfg(feature = "rocksdb_storage")]
            let log_storage = RocksDBStorage::create(&cfg.log_dir, logger.clone())
                .expect("Failed to create storage");

            let raft = Raft::bootstrap(
                node_id,
                options.raft_addr,
                log_storage,
                store.clone(),
                cfg.clone(),
                logger.clone(),
            )?;

            let handle = tokio::spawn(raft.clone().run());
            raft.add_peers(ticket.peers.clone())
                .await
                .expect("Failed to add peers");
            raft.join_cluster(vec![ticket])
                .await
                .expect("Failed to join cluster");

            (raft, handle)
        }
        None => {
            log::info!("Bootstrap a Raft Cluster");

            // NOTE: Due to the characteristic of dynamic bootstrapping,
            // it cannot be bootstrapped directly from the WAL log.
            // Therefore, in this example, we delete the previous logs if they exist and then bootstrap.
            let log_dir = Path::new("./logs");

            if fs::metadata(log_dir).is_ok() {
                fs::remove_dir_all(log_dir)?;
            }

            let leader_node_id = 1;
            let cfg = build_config(leader_node_id, None);

            #[cfg(feature = "inmemory_storage")]
            let log_storage = MemStorage::create();

            #[cfg(feature = "heed_storage")]
            let log_storage = HeedStorage::create(cfg.get_log_dir(), &cfg.clone(), logger.clone())
                .expect("Failed to create storage");

            #[cfg(feature = "rocksdb_storage")]
            let log_storage = RocksDBStorage::create(&storage_pth, logger.clone())
                .expect("Failed to create storage");

            let raft = Raft::bootstrap(
                leader_node_id,
                options.raft_addr,
                log_storage,
                store.clone(),
                cfg,
                logger.clone(),
            )?;
            let handle = tokio::spawn(raft.clone().run());
            (raft, handle)
        }
    };

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
                    .service(leader)
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

    let result = tokio::try_join!(raft_handle)?;
    result.0?;
    Ok(())
}
