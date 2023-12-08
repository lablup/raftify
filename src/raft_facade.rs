use std::collections::HashMap;
use std::net::{SocketAddr, ToSocketAddrs};
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
use tokio::signal;
use tokio::sync::mpsc;
use tonic::Request;

#[derive(Clone)]
pub struct Raft<FSM: AbstractStateMachine + Clone + 'static> {
    pub raft_node: RaftNode<FSM>,
    pub raft_server: RaftServer,
    pub tx: mpsc::Sender<RequestMessage>,
    pub addr: SocketAddr,
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

impl<FSM: AbstractStateMachine + Clone + Send + Sync + 'static> Raft<FSM> {
    pub fn build<A: ToSocketAddrs>(
        node_id: u64,
        addr: A,
        fsm: FSM,
        config: Config,
        logger: slog::Logger,
        initial_peers: Option<Peers>,
    ) -> Result<Self> {
        let addr = addr.to_socket_addrs()?.next().unwrap();
        let initial_peers = initial_peers.unwrap_or_default();

        let (tx, rx) = mpsc::channel(100);

        let bootstrap_done = initial_peers.is_empty();

        let raft_node = match node_id {
            1 => RaftNode::bootstrap_cluster(
                rx,
                tx.clone(),
                fsm,
                config.clone(),
                initial_peers,
                logger.clone(),
                bootstrap_done,
            ),
            _ => RaftNode::new_follower(
                rx,
                tx.clone(),
                node_id,
                fsm,
                config.clone(),
                initial_peers,
                logger.clone(),
                bootstrap_done,
            ),
        }?;

        Ok(Self {
            addr,
            config,
            tx: tx.clone(),
            raft_node,
            raft_server: RaftServer::new(tx.clone(), addr.clone(), logger.clone()),
            logger,
        })
    }

    pub fn mailbox(&self) -> Mailbox {
        Mailbox {
            snd: self.tx.to_owned(),
            peers: HashMap::new(),
        }
    }

    pub async fn run(self) -> Result<()> {
        slog::info!(self.logger, "Start to run RaftNode. Configuration: {:?}", self.config);

        let raft_node = self.raft_node.clone();
        let raft_node_handle = tokio::spawn(async move { raft_node.to_owned().run().await });
        let raft_server = self.raft_server.to_owned();
        let raft_server_handle = tokio::spawn(async move { raft_server.run().await });

        tokio::select! {
            _ = signal::ctrl_c() => {
                slog::info!(self.logger, "Ctrl+C detected. Shutting down...");
                Ok(())
            }
            result = async {
                tokio::try_join!(raft_node_handle, raft_server_handle)
            } => {
                match result {
                    Ok(_) => {
                        slog::debug!(self.logger, "Both tasks completed successfully.");
                        Ok(())
                    },
                    Err(e) => {
                        slog::error!(self.logger, "Error: {:?}", e);
                        Err(Error::Other(Box::new(e)))
                    }
                }
            }
        }
    }

    pub async fn request_id(peer_addr: String) -> Result<RequestIdResponse> {
        println!("Attempting to get a node_id through \"{}\"...", peer_addr);
        let mut leader_addr = peer_addr;

        loop {
            let mut client = RaftServiceClient::connect(format!("http://{}", leader_addr)).await?;
            let response = client
                .request_id(Request::new(RequestIdArgs {}))
                .await?
                .into_inner();

            match response.code() {
                ResultCode::WrongLeader => {
                    leader_addr = response.leader_addr;
                    println!(
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

    pub async fn join(&mut self, request_id_response: RequestIdResponse) -> Result<()> {
        let leader_id = request_id_response.leader_id;
        let leader_addr = request_id_response.leader_addr.clone();
        let reserved_id = request_id_response.reserved_id;

        for (id, peer) in request_id_response.peers.iter() {
            self.raft_node.add_peer(id.to_owned(), peer.addr).await;
        }

        self.raft_node.add_peer(leader_id, leader_addr).await;

        let mut change = ConfChangeV2::default();
        let mut cs = ConfChangeSingle::default();
        cs.set_node_id(reserved_id);
        cs.set_change_type(ConfChangeType::AddNode);
        change.set_changes(vec![cs].into());
        change.set_context(serialize(&vec![self.addr.clone()])?);

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
                ChangeConfigResultType::ChangeConfigTimeoutError => {
                    return Err(Error::Timeout);
                }
            }
        }
    }
}
