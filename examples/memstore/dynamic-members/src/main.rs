#[macro_use]
extern crate slog;
extern crate slog_async;
extern crate slog_scope;
extern crate slog_term;

use example_harness::config::build_config;
use memstore_example_harness::{
    state_machine::{HashStore, LogEntry},
    web_server_api::{get, leader_id, put},
};
use raftify::{raft::derializer::set_custom_deserializer, MyDeserializer, Raft as Raft_};
use slog::Drain;

use actix_web::{web, App, HttpServer};
use slog_envlogger::LogBuilder;
use structopt::StructOpt;
// use slog_envlogger::LogBuilder;

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

    let logger = slog::Logger::root(drain, o!());

    set_custom_deserializer(MyDeserializer::<LogEntry, HashStore>::new());

    // converts log to slog
    // let _scope_guard = slog_scope::set_global_logger(logger.clone());
    let _log_guard = slog_stdlog::init_with_level(log::Level::Debug).unwrap();

    let options = Options::from_args();
    let store = HashStore::new();

    let cfg = build_config();

    let (raft, raft_handle) = match options.peer_addr {
        Some(peer_addr) => {
            log::info!("Running in Follower mode");

            let ticket = Raft::request_id(peer_addr.clone()).await.unwrap();
            let node_id = ticket.reserved_id;

            let raft = Raft::build(
                node_id,
                options.raft_addr,
                store.clone(),
                cfg,
                logger.clone(),
                None,
            )?;

            let handle = tokio::spawn(raft.clone().run());
            raft.join(ticket).await;
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
                None,
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
                    // .service(leave)
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
