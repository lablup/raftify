#[macro_use]
extern crate slog;
extern crate slog_async;
extern crate slog_term;

use actix_web::{web, App, HttpServer};
use raftify::{
    raft::{formatter::set_custom_formatter, logger::Slogger},
    ClusterJoinTicket, CustomFormatter, Raft as Raft_,
};
use slog::Drain;
use slog_envlogger::LogBuilder;
use std::sync::Arc;
use structopt::StructOpt;

use example_harness::config::build_config;
use memstore_example_harness::{
    state_machine::{HashStore, LogEntry},
    web_server_api::{
        campaign, debug, demote, get, leader_id, leave, leave_joint, peers, put, snapshot,
        transfer_leader,
    },
};
use memstore_static_members::utils::load_peers;

type Raft = Raft_<LogEntry, HashStore>;

#[derive(Debug, StructOpt)]
struct Options {
    #[structopt(long)]
    raft_addr: String,
    #[structopt(long)]
    web_server: Option<String>,
    #[structopt(long)]
    restore_wal_from: Option<u64>,
    #[structopt(long)]
    restore_wal_snapshot_from: Option<u64>,
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
    let initial_peers = load_peers().await?;

    let mut cfg = build_config();
    // cfg.initial_peers = Some(initial_peers.clone());
    cfg.restore_wal_from = options.restore_wal_from;
    cfg.restore_wal_snapshot_from = options.restore_wal_snapshot_from;

    let node_id = initial_peers
        .get_node_id_by_addr(options.raft_addr.clone())
        .unwrap();

    let raft = Raft::bootstrap(
        node_id,
        options.raft_addr.clone(),
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
