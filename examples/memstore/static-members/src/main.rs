#[macro_use]
extern crate slog;
extern crate slog_async;
extern crate slog_scope;
extern crate slog_term;

use std::sync::Arc;

use actix_web::{web, App, HttpServer};
use raftify::{
    raft::{formatter::set_custom_formatter, logger::Slogger},
    CustomFormatter, Raft as Raft_,
};
use slog::Drain;
use slog_envlogger::LogBuilder;
use structopt::StructOpt;
// use slog_envlogger::LogBuilder;

use example_harness::config::build_config;
use memstore_example_harness::{
    state_machine::{HashStore, LogEntry},
    web_server_api::{debug, get, leader_id, leave, put},
};
use memstore_static_members::utils::load_peers;

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
    let decorator = slog_term::TermDecorator::new().build();
    let drain = slog_term::FullFormat::new(decorator).build().fuse();
    let drain = slog_async::Async::new(drain).build().fuse();

    let mut builder = LogBuilder::new(drain);
    builder = builder.filter(None, slog::FilterLevel::Debug);

    if let Ok(s) = std::env::var("RUST_LOG") {
        builder = builder.parse(&s);
    }
    let drain = builder.build();

    let logger = Arc::new(Slogger {
        slog: slog::Logger::root(drain, o!()),
    });

    set_custom_formatter(CustomFormatter::<LogEntry, HashStore>::new());

    // converts log to slog
    // let _scope_guard = slog_scope::set_global_logger(logger.clone());
    #[allow(clippy::let_unit_value)]
    let _log_guard = slog_stdlog::init_with_level(log::Level::Debug).unwrap();

    let options = Options::from_args();
    let store = HashStore::new();
    let peers = load_peers().await?;

    let cfg = build_config();

    let (raft, raft_handle) = match options.peer_addr {
        Some(_peer_addr) => {
            log::info!("Running in Follower mode");

            let node_id = peers
                .get_node_id_by_addr(options.raft_addr.clone())
                .unwrap();

            let raft = Raft::build(
                node_id,
                options.raft_addr,
                store.clone(),
                cfg,
                logger.clone(),
                Some(peers.clone()),
            )?;

            let handle = tokio::spawn(raft.clone().run());

            let leader_addr = peers.get(&1).unwrap().addr;
            Raft::member_bootstrap_ready(leader_addr, node_id).await?;

            (raft, handle)
        }
        None => {
            log::info!("Bootstrap a Raft Cluster");
            let node_id = 1;
            let raft = Raft::build(
                node_id,
                options.raft_addr,
                store.clone(),
                cfg,
                logger.clone(),
                Some(peers),
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
                    .service(leader_id)
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
