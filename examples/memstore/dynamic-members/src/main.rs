#[macro_use]
extern crate slog;
extern crate slog_async;
extern crate slog_term;

use actix_web::{web, App, HttpServer};
use raftify::{
    raft::{formatter::set_custom_formatter, logger::Slogger},
    CustomFormatter, Raft as Raft_,
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

type Raft = Raft_<LogEntry, HashStore>;

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

    let mut cfg = build_config();

    let (raft, raft_handle) = match options.peer_addr {
        Some(peer_addr) => {
            log::info!("Running in Follower mode");

            let ticket = Raft::request_id(options.raft_addr.clone(), peer_addr.clone())
                .await
                .unwrap();
            let node_id = ticket.reserved_id;
            cfg.initial_peers = Some(ticket.peers.clone().into());

            let raft = Raft::bootstrap(
                node_id,
                options.raft_addr,
                store.clone(),
                cfg.clone(),
                logger.clone(),
            )?;

            let handle = tokio::spawn(raft.clone().run());
            raft.add_peers(ticket.peers.clone()).await;
            raft.join_cluster(vec![ticket]).await;

            (raft, handle)
        }
        None => {
            log::info!("Bootstrap a Raft Cluster");
            let raft = Raft::bootstrap(1, options.raft_addr, store.clone(), cfg, logger.clone())?;
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

    let result = tokio::try_join!(raft_handle)?;
    result.0?;
    Ok(())
}
