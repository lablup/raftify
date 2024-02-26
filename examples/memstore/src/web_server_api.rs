use std::collections::HashMap;

use actix_web::{get, web, Responder};
use raftify::{
    create_client, raft_service, AbstractLogEntry, ClusterJoinTicket, Peers, Raft as Raft_,
};
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
    raft.1.capture_snapshot().await.unwrap();
    "OK".to_string()
}

#[get("/leave_joint")]
async fn leave_joint(data: web::Data<(HashStore, Raft)>) -> impl Responder {
    let raft = data.clone();
    raft.1.raft_node.leave_joint().await;
    "OK".to_string()
}

#[get("/transfer_leader/{id}")]
async fn transfer_leader(
    data: web::Data<(HashStore, Raft)>,
    path: web::Path<u64>,
) -> impl Responder {
    let raft = data.clone();
    let node_id: u64 = path.into_inner();
    raft.1.raft_node.transfer_leader(node_id).await;
    "OK".to_string()
}

#[get("/campaign")]
async fn campaign(data: web::Data<(HashStore, Raft)>) -> impl Responder {
    let raft = data.clone();
    raft.1.raft_node.campaign().await;
    "OK".to_string()
}

#[get("/demote/{term}/{leader_id}")]
async fn demote(data: web::Data<(HashStore, Raft)>, path: web::Path<(u64, u64)>) -> impl Responder {
    let raft = data.clone();
    let (term, leader_id_) = path.into_inner();
    raft.1.raft_node.demote(term, leader_id_).await;
    "OK".to_string()
}
