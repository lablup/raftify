use std::{collections::HashMap, os::unix::net::SocketAddr};

use actix_web::{get, web, Responder};
use raftify::{
    create_client, raft_service, AbstractLogEntry, ClusterJoinTicket, Peers, Raft as Raft_,
    RaftServiceClient,
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

// TODO: Investigate why auto type joint consensus is not closed.
#[get("/join_test")]
async fn join_test(data: web::Data<(HashStore, Raft)>) -> impl Responder {
    let raft = data.clone();
    let mut initial_peers = Peers::with_empty();
    initial_peers.add_peer(1, "127.0.0.1:60061", None);
    initial_peers.add_peer(2, "127.0.0.1:60062", None);
    initial_peers.add_peer(3, "127.0.0.1:60063", None);

    let ticket1 = ClusterJoinTicket {
        reserved_id: 1,
        raft_addr: "127.0.0.1:60061".to_owned(),
        leader_id: 1,
        leader_addr: "127.0.0.1:60061".to_owned(),
        peers: initial_peers.clone().into(),
    };

    let ticket2 = ClusterJoinTicket {
        reserved_id: 2,
        raft_addr: "127.0.0.1:60062".to_owned(),
        leader_id: 1,
        leader_addr: "127.0.0.1:60061".to_owned(),
        peers: initial_peers.clone().into(),
    };

    let ticket3 = ClusterJoinTicket {
        reserved_id: 3,
        raft_addr: "127.0.0.1:60063".to_owned(),
        leader_id: 1,
        leader_addr: "127.0.0.1:60061".to_owned(),
        peers: initial_peers.clone().into(),
    };

    raft.1.join(vec![ticket1, ticket2, ticket3]).await;

    let mut client_2 = create_client(&"127.0.0.1:60062").await.unwrap();

    client_2
        .set_peers(raft_service::Peers {
            peers: vec![
                raft_service::Peer {
                    node_id: 1,
                    addr: "127.0.0.1:60061".to_owned(),
                },
                raft_service::Peer {
                    node_id: 2,
                    addr: "127.0.0.1:60062".to_owned(),
                },
                raft_service::Peer {
                    node_id: 3,
                    addr: "127.0.0.1:60063".to_owned(),
                },
            ],
        })
        .await
        .unwrap();

    let mut client_3 = create_client(&"127.0.0.1:60063").await.unwrap();

    client_3
        .set_peers(raft_service::Peers {
            peers: vec![
                raft_service::Peer {
                    node_id: 1,
                    addr: "127.0.0.1:60061".to_owned(),
                },
                raft_service::Peer {
                    node_id: 2,
                    addr: "127.0.0.1:60062".to_owned(),
                },
                raft_service::Peer {
                    node_id: 3,
                    addr: "127.0.0.1:60063".to_owned(),
                },
            ],
        })
        .await
        .unwrap();

    "OK".to_string()
}
