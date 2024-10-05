use std::collections::HashMap;

use actix_web::{get, post, put, web, HttpResponse, Responder};
use raftify::{raft::Storage, AbstractLogEntry, StableStorage};
use serde_json::Value;

use crate::state_machine::{HashStore, LogEntry, Raft};

#[put("/store/{key}/{value}")]
async fn put(data: web::Data<(HashStore, Raft)>, path: web::Path<(u64, String)>) -> impl Responder {
    let log_entry = LogEntry::Insert {
        key: path.0,
        value: path.1.clone(),
    };
    data.1.propose(log_entry.encode().unwrap()).await.unwrap();

    "OK".to_string()
}

#[get("/store/{key}")]
async fn get(data: web::Data<(HashStore, Raft)>, path: web::Path<u64>) -> impl Responder {
    let key = path.into_inner();

    match data.0.get(key) {
        Some(value) => HttpResponse::Ok().body(value),
        None => HttpResponse::BadRequest().body("Bad Request: Item not found"),
    }
}

#[get("/leader")]
async fn leader(data: web::Data<(HashStore, Raft)>) -> impl Responder {
    let raft = data.clone();
    let leader_id = raft
        .1
        .get_leader_id()
        .await
        .expect("Failed to get leader id")
        .to_string();
    format!("{:?}", leader_id)
}

#[post("/leave")]
async fn leave(data: web::Data<(HashStore, Raft)>) -> impl Responder {
    let raft = data.clone();
    raft.1.leave().await.unwrap();
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

#[post("/snapshot")]
async fn snapshot(data: web::Data<(HashStore, Raft)>) -> impl Responder {
    #[cfg(feature = "inmemory_storage")]
    {
        return "Snapshot is not supported with inmemory storage".to_string();
    }

    #[cfg(not(feature = "inmemory_storage"))]
    {
        let raft = data.clone();
        let last_index = raft
            .1
            .storage()
            .await
            .expect("Failed to get storage")
            .last_index()
            .expect("Failed to get last index");

        let hard_state = raft
            .1
            .storage()
            .await
            .expect("Failed to get storage")
            .hard_state()
            .expect("Failed to get hard state");

        raft.1
            .make_snapshot(last_index, hard_state.term)
            .await
            .expect("Failed to make snapshot");
        "OK".to_string()
    }
}

#[post("/leave_joint")]
async fn leave_joint(data: web::Data<(HashStore, Raft)>) -> impl Responder {
    let raft = data.clone();
    raft.1.leave_joint().await;
    "OK".to_string()
}

#[post("/transfer_leader/{id}")]
async fn transfer_leader(
    data: web::Data<(HashStore, Raft)>,
    path: web::Path<u64>,
) -> impl Responder {
    let raft = data.clone();
    let node_id: u64 = path.into_inner();
    raft.1.transfer_leader(node_id).await.unwrap();
    "OK".to_string()
}

#[post("/campaign")]
async fn campaign(data: web::Data<(HashStore, Raft)>) -> impl Responder {
    let raft = data.clone();
    raft.1.campaign().await.unwrap();
    "OK".to_string()
}

#[post("/demote/{term}/{leader_id}")]
async fn demote(data: web::Data<(HashStore, Raft)>, path: web::Path<(u64, u64)>) -> impl Responder {
    let raft = data.clone();
    let (term, leader_id) = path.into_inner();
    raft.1.demote(term, leader_id).await.unwrap();
    "OK".to_string()
}
