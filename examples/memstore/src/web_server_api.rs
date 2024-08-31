use std::collections::HashMap;

use actix_web::{get, put, web, Responder};
use raftify::{raft::Storage, AbstractLogEntry, Raft as Raft_, StableStorage};
use serde_json::Value;

use super::state_machine::{HashStore, LogEntry};

type Raft = Raft_<LogEntry, HashStore>;

#[put("/store/{id}/{value}")]
async fn put(data: web::Data<(HashStore, Raft)>, path: web::Path<(u64, String)>) -> impl Responder {
    let log_entry = LogEntry::Insert {
        key: path.0,
        value: path.1.clone(),
    };
    data.1.propose(log_entry.encode().unwrap()).await.unwrap();

    "OK".to_string()
}

#[get("/store/{id}")]
async fn get(data: web::Data<(HashStore, Raft)>, path: web::Path<u64>) -> impl Responder {
    let id = path.into_inner();

    let response = data.0.get(id);
    format!("{:?}", response)
}

#[get("/leader")]
async fn leader_id(data: web::Data<(HashStore, Raft)>) -> impl Responder {
    let raft = data.clone();
    let leader_id = raft.1.get_leader_id().await.to_string();
    format!("{:?}", leader_id)
}

#[get("/leave")]
async fn leave(data: web::Data<(HashStore, Raft)>) -> impl Responder {
    let raft = data.clone();
    raft.1.leave().await;
    "OK".to_string()
}

#[get("/debug")]
async fn debug(data: web::Data<(HashStore, Raft)>) -> impl Responder {
    let raft = data.clone();
    let json = raft.1.inspect().await.unwrap();
    let parsed: HashMap<String, Value> = serde_json::from_str(&json).unwrap();
    format!("{:?}", parsed)
}

#[get("/peers")]
async fn peers(data: web::Data<(HashStore, Raft)>) -> impl Responder {
    let raft = data.clone();
    let peers = raft.1.get_peers().await;
    format!("{:?}", peers)
}

#[get("/snapshot")]
async fn snapshot(data: web::Data<(HashStore, Raft)>) -> impl Responder {
    let raft = data.clone();
    let last_index = raft
        .1
        .storage()
        .await
        .last_index()
        .expect("Failed to get last index");

    let hard_state = raft
        .1
        .storage()
        .await
        .hard_state()
        .expect("Failed to get hard state");

    raft.1.make_snapshot(last_index, hard_state.term).await;
    "OK".to_string()
}

#[get("/leave_joint")]
async fn leave_joint(data: web::Data<(HashStore, Raft)>) -> impl Responder {
    let raft = data.clone();
    raft.1.leave_joint().await;
    "OK".to_string()
}

#[get("/transfer_leader/{id}")]
async fn transfer_leader(
    data: web::Data<(HashStore, Raft)>,
    path: web::Path<u64>,
) -> impl Responder {
    let raft = data.clone();
    let node_id: u64 = path.into_inner();
    raft.1.transfer_leader(node_id).await;
    "OK".to_string()
}

#[get("/campaign")]
async fn campaign(data: web::Data<(HashStore, Raft)>) -> impl Responder {
    let raft = data.clone();
    raft.1.campaign().await;
    "OK".to_string()
}

#[get("/demote/{term}/{leader_id}")]
async fn demote(data: web::Data<(HashStore, Raft)>, path: web::Path<(u64, u64)>) -> impl Responder {
    let raft = data.clone();
    let (term, leader_id_) = path.into_inner();
    raft.1.demote(term, leader_id_).await;
    "OK".to_string()
}
