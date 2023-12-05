use std::collections::HashMap;
use std::sync::Arc;

use crate::error::{Error, Result};
use crate::raft_node::RaftNode;
use crate::raft_server::RaftServer;
use crate::raft_service::raft_service_client::RaftServiceClient;
use crate::raft_service::{ChangeConfigResultType, RequestIdArgs, ResultCode};
use crate::request_message::RequestMessage;
use crate::{AbstractStateMachine, Config, Mailbox, Peer, Peers};

use bincode::{deserialize, serialize};
use log::info;
use raft::eraftpb::{ConfChangeSingle, ConfChangeType, ConfChangeV2};
use tokio::sync::{mpsc, Mutex};
use tonic::Request;

#[derive(Clone)]
pub struct Raft<FSM: AbstractStateMachine + 'static> {
    pub raft_node: Option<RaftNode<FSM>>,
    pub raft_server: Option<RaftServer>,
    pub fsm: Option<FSM>,
    pub initial_peers: Option<Peers>,
    pub tx: Option<mpsc::Sender<RequestMessage>>,
    pub addr: String,
    pub logger: slog::Logger,
    pub config: Config,
}

#[derive(Debug, Clone)]
pub struct RequestIdResponse {
    pub reserved_id: u64,
    pub leader_id: u64,
    pub leader_addr: String,
    pub peers: HashMap<u64, Peer>,
}

impl<S: AbstractStateMachine + Send + Sync + 'static> Raft<S> {
    /// creates a new node with the given address and store.
    pub fn new(addr: String, fsm: S, config: Config, logger: slog::Logger) -> Self {
        Self {
            addr,
            logger,
            config,
            tx: None,
            fsm: Some(fsm),
            raft_node: None,
            raft_server: None,
            initial_peers: Some(Peers::new()),
        }
    }

    pub fn mailbox(&self) -> Mailbox {
        assert!(self.is_initialized());

        Mailbox {
            snd: self.tx.to_owned().unwrap(),
            peers: HashMap::new(),
        }
    }

    pub fn is_initialized(&self) -> bool {
        self.raft_node.is_some() && self.raft_server.is_some()
    }

    pub fn build(&mut self, node_id: u64) -> Result<()> {
        let (tx, rx) = mpsc::channel(100);
        self.tx = Some(tx.clone());

        let bootstrap_done = self.initial_peers.clone().unwrap().is_empty();

        let raft_node = match node_id {
            1 => RaftNode::bootstrap_cluster(
                rx,
                tx.clone(),
                self.fsm.take().unwrap(),
                self.config.clone(),
                self.initial_peers.take().unwrap(),
                &self.logger,
                bootstrap_done,
            ),
            _ => RaftNode::new_follower(
                rx,
                tx.clone(),
                node_id,
                self.fsm.take().unwrap(),
                self.config.clone(),
                self.initial_peers.take().unwrap(),
                &self.logger,
                bootstrap_done,
            ),
        }?;

        self.raft_node = Some(raft_node);
        let raft_server = RaftServer::new(tx.clone(), self.addr.clone());
        self.raft_server = Some(raft_server);
        Ok(())
    }

    pub async fn run(self) -> Result<()> {
        assert!(self.is_initialized());

        let raft_node = self.raft_node.to_owned().unwrap();
        let raft_node_handle = tokio::spawn(async move { raft_node.run() });
        let raft_server = self.raft_server.to_owned().unwrap();
        let _raft_server_handle = tokio::spawn(async move { raft_server.run() });
        let _ = tokio::try_join!(raft_node_handle);
        Ok(())
    }

    pub async fn request_id(&self, peer_addr: String) -> Result<RequestIdResponse> {
        info!("Attempting to get a node_id through \"{}\"...", peer_addr);
        let mut leader_addr = peer_addr;

        loop {
            let mut client = RaftServiceClient::connect(format!("http://{}", leader_addr)).await?;
            let response = client
                .request_id(Request::new(RequestIdArgs {
                    addr: self.addr.clone(),
                }))
                .await?
                .into_inner();

            match response.code() {
                ResultCode::WrongLeader => {
                    leader_addr = response.leader_addr;
                    info!(
                        "Sent message to the wrong leader, retrying with leader at {}",
                        leader_addr
                    );
                    continue;
                }
                ResultCode::Ok => {
                    break {
                        Ok(RequestIdResponse {
                            reserved_id: response.reserved_id,
                            leader_id: response.leader_id,
                            leader_addr: response.leader_addr,
                            peers: deserialize(&response.peers)?,
                        })
                    };
                }
                ResultCode::Error => return Err(Error::JoinError),
            }
        }
    }

    pub async fn join(&self, request_id_response: RequestIdResponse) -> Result<()> {
        assert!(self.is_initialized());
        let node = self.raft_node.to_owned().unwrap();
        let mut node = node.lock().await;

        let leader_id = request_id_response.leader_id;
        let leader_addr = request_id_response.leader_addr.clone();
        let reserved_id = request_id_response.reserved_id;

        for (id, peer) in request_id_response.peers.iter() {
            node.peers.add_peer(id.to_owned(), peer.addr);
        }
        node.peers.add_peer(leader_id, leader_addr);

        let mut change = ConfChangeV2::default();
        let mut cs = ConfChangeSingle::default();
        cs.set_node_id(reserved_id);
        cs.set_change_type(ConfChangeType::AddNode);
        change.set_changes(vec![cs].into());
        change.set_context(serialize(&self.addr)?);

        let peer_addr = request_id_response.leader_addr;

        loop {
            let mut leader_client =
                RaftServiceClient::connect(format!("http://{}", peer_addr)).await?;
            let response = leader_client
                .change_config(Request::new(change.clone()))
                .await?
                .into_inner();

            match response.result_type() {
                ChangeConfigResultType::ChangeConfigWrongLeader => {
                    // TODO: Handle this
                    // response.data();
                    continue;
                }
                ChangeConfigResultType::ChangeConfigSuccess => break Ok(()),
                ChangeConfigResultType::ChangeConfigUnknownError => return Err(Error::JoinError),
                ChangeConfigResultType::ChangeConfigRejected => {
                    return Err(Error::Rejected("Join request rejected".to_string()))
                }
                ChangeConfigResultType::ChangeConfigTimeoutError => return Err(Error::Timeout),
            }
        }
    }
}
