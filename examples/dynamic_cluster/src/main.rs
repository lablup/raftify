#[macro_use]
extern crate slog;
extern crate slog_async;
extern crate slog_scope;
extern crate slog_term;

use dynamic_cluster::utils::build_config;
use raftify::raft::derializer::set_custom_deserializer;
use slog::Drain;

use actix_web::{get, web, App, HttpServer, Responder};
use async_trait::async_trait;
use bincode::{deserialize, serialize};
use raftify::{AbstractLogEntry, AbstractStateMachine, Mailbox, MyDeserializer, Raft, Result};
use serde::{Deserialize, Serialize};
use slog_envlogger::LogBuilder;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use structopt::StructOpt;
// use slog_envlogger::LogBuilder;

#[derive(Debug, StructOpt)]
struct Options {
    #[structopt(long)]
    raft_addr: String,
    #[structopt(long)]
    peer_addr: Option<String>,
    #[structopt(long)]
    web_server: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum LogEntry {
    Insert { key: u64, value: String },
}

impl AbstractLogEntry for LogEntry {
    fn encode(&self) -> Result<Vec<u8>> {
        serialize(self).map_err(|e| e.into())
    }

    fn decode(bytes: &[u8]) -> Result<LogEntry> {
        let log_entry: LogEntry = deserialize(bytes)?;
        Ok(log_entry)
    }
}

#[derive(Clone, Debug)]
struct HashStore(Arc<RwLock<HashMap<u64, String>>>);

impl HashStore {
    fn new() -> Self {
        Self(Arc::new(RwLock::new(HashMap::new())))
    }
    fn get(&self, id: u64) -> Option<String> {
        self.0.read().unwrap().get(&id).cloned()
    }
}

#[async_trait]
impl AbstractStateMachine<LogEntry> for HashStore {
    async fn apply(&mut self, log_entry: LogEntry) -> Result<LogEntry> {
        match log_entry {
            LogEntry::Insert { ref key, ref value } => {
                let mut db = self.0.write().unwrap();
                log::info!("Inserted: ({}, {})", key, value);
                db.insert(*key, value.clone());
            }
        };
        Ok(log_entry)
    }

    async fn snapshot(&self) -> Result<HashStore> {
        let snapshot = self.0.read().unwrap().clone();
        Ok(HashStore(Arc::new(RwLock::new(snapshot))))
    }

    async fn restore(&mut self, snapshot: HashStore) -> Result<()> {
        let new: HashMap<u64, String> = snapshot.0.read().unwrap().clone();
        let mut db = self.0.write().unwrap();
        let _ = std::mem::replace(&mut *db, new);
        Ok(())
    }

    fn encode(&self) -> Result<Vec<u8>> {
        serialize(&self.0.read().unwrap().clone()).map_err(|e| e.into())
    }

    fn decode(bytes: &[u8]) -> Result<Self> {
        let db: HashMap<u64, String> = deserialize(bytes)?;
        Ok(Self(Arc::new(RwLock::new(db))))
    }
}

#[get("/put/{id}/{name}")]
async fn put(
    data: web::Data<(Arc<Mailbox>, HashStore, Raft<LogEntry, HashStore>)>,
    path: web::Path<(u64, String)>,
) -> impl Responder {
    let log_entry = LogEntry::Insert {
        key: path.0,
        value: path.1.clone(),
    };
    let log_entry = serialize(&log_entry).unwrap();
    let result = data.0.send(log_entry).await.unwrap();

    let result: LogEntry = deserialize(&result).unwrap();
    format!("{:?}", result)
}

#[get("/get/{id}")]
async fn get(
    data: web::Data<(Arc<Mailbox>, HashStore, Raft<LogEntry, HashStore>)>,
    path: web::Path<u64>,
) -> impl Responder {
    let id = path.into_inner();

    let response = data.1.get(id);
    format!("{:?}", response)
}

#[get("/leave")]
async fn leave(
    data: web::Data<(Arc<Mailbox>, HashStore, Raft<LogEntry, HashStore>)>,
) -> impl Responder {
    data.0.leave().await.unwrap();
    "OK".to_string()
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
            let request_id_resp =
                Raft::<LogEntry, HashStore>::request_id(peer_addr.clone()).await?;
            let mut raft = Raft::build(
                request_id_resp.reserved_id,
                options.raft_addr,
                store.clone(),
                cfg,
                logger.clone(),
                None,
            )?;
            let handle = tokio::spawn(raft.clone().run());
            raft.join(request_id_resp).await?;
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

    let mailbox = Arc::new(raft.mailbox());

    if let Some(addr) = options.web_server {
        let _web_server = tokio::spawn(
            HttpServer::new(move || {
                App::new()
                    .app_data(web::Data::new((
                        mailbox.clone(),
                        store.clone(),
                        raft.clone(),
                    )))
                    .service(put)
                    .service(get)
                    .service(leave)
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
