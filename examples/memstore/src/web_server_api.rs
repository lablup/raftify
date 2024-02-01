use std::collections::HashMap;

use actix_web::{get, web, Responder};
use raftify::{AbstractLogEntry, Raft as Raft_};
use serde_json::Value;

use super::state_machine::{HashStore, LogEntry};

type Raft = Raft_<LogEntry, HashStore>;

#[get("/put/{id}/{value}")]
async fn put(data: web::Data<(HashStore, Raft)>, path: web::Path<(u64, String)>) -> impl Responder {
    let log_entry = LogEntry::Insert {
        key: path.0,
        value: path.1.clone(),
    };
    data.1
        .raft_node
        .propose(log_entry.encode().unwrap())
        .await
        .unwrap();

    "OK".to_string()
}

#[get("/get/{id}")]
async fn get(data: web::Data<(HashStore, Raft)>, path: web::Path<u64>) -> impl Responder {
    let id = path.into_inner();

    let response = data.0.get(id);
    format!("{:?}", response)
}

#[get("/leader")]
async fn leader_id(data: web::Data<(HashStore, Raft)>) -> impl Responder {
    let raft = data.clone();
    let leader_id = raft.1.raft_node.get_leader_id().await.to_string();
    format!("{:?}", leader_id)
}

#[get("/leave")]
async fn leave(data: web::Data<(HashStore, Raft)>) -> impl Responder {
    let raft = data.clone();
    raft.1.raft_node.clone().leave().await;
    "OK".to_string()
}

#[get("/debug")]
async fn debug(data: web::Data<(HashStore, Raft)>) -> impl Responder {
    let raft = data.clone();
    let json = raft.1.raft_node.clone().inspect().await.unwrap();
    let parsed: HashMap<String, Value> = serde_json::from_str(&json).unwrap();
    format!("{:?}", parsed)
}

#[get("/peers")]
async fn peers(data: web::Data<(HashStore, Raft)>) -> impl Responder {
    let raft = data.clone();
    let peers = raft.1.raft_node.clone().get_peers().await;
    format!("{:?}", peers)
}

#[get("/snapshot")]
async fn snapshot(data: web::Data<(HashStore, Raft)>) -> impl Responder {
    let raft = data.clone();
    raft.1.snapshot().await.unwrap();
    "OK".to_string()
}
