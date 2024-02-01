use bincode::deserialize;
use jopemachine_raft::logger::Logger;
use std::{
    collections::HashMap,
    net::{SocketAddr, ToSocketAddrs},
    sync::Arc,
};
use tokio::{
    signal,
    sync::{mpsc, oneshot},
};

use crate::{InitialRole, Peers};

use super::{
    create_client,
    error::{Error, Result},
    raft_node::RaftNode,
    raft_server::RaftServer,
    raft_service::{self, ResultCode},
    request_message::ServerRequestMsg,
    AbstractLogEntry, AbstractStateMachine, Config, LogStore,
};

#[derive(Clone)]
pub struct Raft<LogEntry: AbstractLogEntry + 'static, FSM: AbstractStateMachine + Clone + 'static> {
    pub raft_node: RaftNode<LogEntry, FSM>,
    pub raft_server: RaftServer,
    pub server_tx: mpsc::Sender<ServerRequestMsg>,
    pub raft_addr: SocketAddr,
    pub logger: Arc<dyn Logger>,
    pub config: Config,
}

#[derive(Debug, Clone)]
pub struct ClusterJoinTicket {
    pub reserved_id: u64,
    pub leader_id: u64,
    pub leader_addr: String,
    pub peers: HashMap<u64, SocketAddr>,
}

impl<LogEntry: AbstractLogEntry, FSM: AbstractStateMachine + Clone + Send + Sync + 'static>
    Raft<LogEntry, FSM>
{
    pub fn bootstrap<A: ToSocketAddrs>(
        node_id: u64,
        raft_addr: A,
        fsm: FSM,
        config: Config,
        logger: Arc<dyn Logger>,
    ) -> Result<Self> {
        let raft_addr = raft_addr.to_socket_addrs()?.next().unwrap();
        let mut should_be_leader = config.initial_peers.is_none();

        if config.initial_peers.is_some() {
            let leaders = config
                .initial_peers
                .clone()
                .unwrap()
                .inner
                .into_iter()
                .filter(|(_, peer)| peer.role == InitialRole::Leader)
                .map(|(key, _)| key)
                .collect::<Vec<_>>();

            assert!(leaders.len() <= 2);
            should_be_leader = leaders.contains(&node_id);
        }

        let (local_tx, local_rx) = mpsc::channel(100);
        let (server_tx, server_rx) = mpsc::channel(100);

        let raft_node = RaftNode::bootstrap(
            node_id,
            should_be_leader,
            fsm,
            config.clone(),
            raft_addr,
            logger.clone(),
            server_rx,
            server_tx.clone(),
            local_rx,
            local_tx.clone(),
        )?;

        let raft_server =
            RaftServer::new(server_tx.clone(), raft_addr, config.clone(), logger.clone());

        Ok(Self {
            raft_addr,
            server_tx: server_tx.clone(),
            raft_node,
            raft_server,
            config,
            logger,
        })
    }

    pub async fn run(self) -> Result<()> {
        self.logger
            .info(&format!("Start to run RaftNode. {:?}", self.config));

        let (quit_signal_tx, quit_signal_rx) = oneshot::channel::<()>();

        let raft_node = self.raft_node.clone();
        let raft_node_handle = tokio::spawn(raft_node.run());
        let raft_server = self.raft_server.clone();
        let raft_server_handle = tokio::spawn(raft_server.run(quit_signal_rx));

        tokio::select! {
            _ = signal::ctrl_c() => {
                self.logger.info("Ctrl+C signal detected. Shutting down...");
                Ok(())
            }
            result = raft_node_handle => {
                quit_signal_tx.send(()).unwrap();

                match result {
                    Ok(raft_node_result) => {
                        match raft_node_result {
                            Ok(_) => {
                                self.logger.info("RaftNode quitted. Shutting down...");
                                Ok(())
                            },
                            Err(err) => {
                                self.logger.error(&format!("RaftNode quitted with the error. Shutting down... {:?}", err));
                                Err(Error::Other(Box::new(err)))
                            }
                        }
                    },
                    Err(err) => {
                        self.logger.error(&format!("RaftNode quitted with the error. Shutting down... {:?}", err));
                        Err(Error::Unknown)
                    }
                }
            }
            result = raft_server_handle => {
                match result {
                    Ok(raft_server_result) => {
                        match raft_server_result {
                            Ok(_) => {
                                self.logger.info("RaftServer quitted. Shutting down...");
                                Ok(())
                            },
                            Err(err) => {
                                self.logger.error(&format!("RaftServer quitted with error. Shutting down... {:?}", err));
                                Err(Error::Other(Box::new(err)))
                            }
                        }
                    },
                    Err(err) => {
                        self.logger.error(&format!("RaftServer quitted with the error. Shutting down... {:?}", err));
                        Err(Error::Unknown)
                    }
                }
            }
        }
    }

    pub async fn request_id<A: ToSocketAddrs>(
        raft_addr: A,
        peer_addr: String,
    ) -> Result<ClusterJoinTicket> {
        let raft_addr = raft_addr
            .to_socket_addrs()
            .unwrap()
            .next()
            .unwrap()
            .to_string();

        let mut client = create_client(&peer_addr).await?;
        let response = client
            .request_id(raft_service::RequestIdArgs {
                raft_addr: raft_addr.to_string(),
            })
            .await?
            .into_inner();

        let peers: Peers = deserialize(&response.peers)?;
        match response.code() {
            ResultCode::Ok => Ok(ClusterJoinTicket {
                reserved_id: response.reserved_id,
                leader_id: response.leader_id,
                leader_addr: response.leader_addr,
                peers: peers.into(),
            }),
            ResultCode::Error => Err(Error::JoinError),
            ResultCode::WrongLeader => {
                unreachable!();
            }
        }
    }

    pub async fn capture_snapshot(&self) -> Result<()> {
        let storage = self.raft_node.storage().await;

        self.raft_node
            .make_snapshot(storage.last_index()?, storage.hard_state()?.term)
            .await;
        Ok(())
    }

    pub async fn join(&self, ticket: ClusterJoinTicket) {
        self.raft_node.join_cluster(ticket).await;
    }
}
