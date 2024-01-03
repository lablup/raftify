use bincode::deserialize;
use std::collections::HashMap;
use std::net::{SocketAddr, ToSocketAddrs};
use std::time::Duration;
use tokio::{
    signal,
    sync::{mpsc, oneshot},
    time::sleep,
};
use tonic::Request;

use super::error::{Error, Result};
use super::raft_node::RaftNode;
use super::raft_server::RaftServer;
use super::raft_service::{self, MemberBootstrapReadyArgs, ResultCode};
use super::request_message::ServerRequestMsg;
use super::{create_client, AbstractLogEntry, AbstractStateMachine, Config, LogStore, Peers};

#[derive(Clone)]
pub struct Raft<LogEntry: AbstractLogEntry + 'static, FSM: AbstractStateMachine + Clone + 'static> {
    pub raft_node: RaftNode<LogEntry, FSM>,
    pub raft_server: RaftServer,
    pub server_tx: mpsc::Sender<ServerRequestMsg>,
    pub raft_addr: SocketAddr,
    pub logger: slog::Logger,
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
    pub fn build<A: ToSocketAddrs>(
        node_id: u64,
        raft_addr: A,
        fsm: FSM,
        config: Config,
        logger: slog::Logger,
        initial_peers: Option<Peers>,
    ) -> Result<Self> {
        let raft_addr = raft_addr.to_socket_addrs()?.next().unwrap();
        let initial_peers = initial_peers.unwrap_or(Peers::new(node_id, raft_addr));

        let (local_tx, local_rx) = mpsc::channel(100);
        let (server_tx, server_rx) = mpsc::channel(100);
        let bootstrap_done = initial_peers.is_empty() || initial_peers.len() <= 1;

        let raft_node = match node_id {
            1 => RaftNode::bootstrap_cluster(
                fsm,
                config.clone(),
                initial_peers,
                raft_addr,
                logger.clone(),
                bootstrap_done,
                server_rx,
                server_tx.clone(),
                local_rx,
                local_tx.clone(),
            ),
            _ => RaftNode::new_follower(
                node_id,
                fsm,
                config.clone(),
                initial_peers,
                raft_addr,
                logger.clone(),
                bootstrap_done,
                server_rx,
                server_tx.clone(),
                local_rx,
                local_tx.clone(),
            ),
        }?;

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
        slog::info!(self.logger, "Start to run RaftNode. {:?}", self.config);

        let (quit_signal_tx, quit_signal_rx) = oneshot::channel::<()>();

        let raft_node = self.raft_node.clone();
        let raft_node_handle = tokio::spawn(raft_node.run());
        let raft_server = self.raft_server.clone();
        let raft_server_handle = tokio::spawn(raft_server.run(quit_signal_rx));

        tokio::select! {
            _ = signal::ctrl_c() => {
                slog::info!(self.logger, "Ctrl+C signal detected. Shutting down...");
                Ok(())
            }
            result = raft_node_handle => {
                quit_signal_tx.send(()).unwrap();

                match result {
                    Ok(raft_node_result) => {
                        match raft_node_result {
                            Ok(_) => {
                                slog::info!(self.logger, "RaftNode quitted. Shutting down...");
                                Ok(())
                            },
                            Err(err) => {
                                slog::error!(self.logger, "RaftNode quitted with error. Shutting down... {:?}", err);
                                Err(Error::Other(Box::new(err)))
                            }
                        }
                    },
                    Err(err) => {
                        slog::error!(self.logger, "RaftNode quitted with join error. Shutting down... {:?}", err);
                        Err(Error::Unknown)
                    }
                }
            }
            result = raft_server_handle => {
                match result {
                    Ok(raft_server_result) => {
                        match raft_server_result {
                            Ok(_) => {
                                slog::info!(self.logger, "RaftServer quitted. Shutting down...");
                                Ok(())
                            },
                            Err(err) => {
                                slog::error!(self.logger, "RaftServer quitted with error. Shutting down... {:?}", err);
                                Err(Error::Other(Box::new(err)))
                            }
                        }
                    },
                    Err(err) => {
                        slog::error!(self.logger, "RaftServer quitted with join error. Shutting down... {:?}", err);
                        Err(Error::Unknown)
                    }
                }
            }
        }
    }

    pub async fn request_id(peer_addr: String) -> Result<ClusterJoinTicket> {
        println!("Attempting to get a node_id through \"{}\"...", peer_addr);
        let mut leader_addr = peer_addr;

        loop {
            let mut client = create_client(&leader_addr).await?;
            let response = client
                .request_id(Request::new(raft_service::Empty {}))
                .await?
                .into_inner();

            match response.code() {
                ResultCode::WrongLeader => {
                    leader_addr = response.leader_addr;
                    println!(
                        "Sent message to the wrong leader, retrying with the leader at {}.",
                        leader_addr
                    );
                    continue;
                }
                ResultCode::Ok => {
                    return Ok(ClusterJoinTicket {
                        reserved_id: response.reserved_id,
                        leader_id: response.leader_id,
                        leader_addr: response.leader_addr,
                        peers: deserialize(&response.peers)?,
                    });
                }
                ResultCode::Error => return Err(Error::JoinError),
            }
        }
    }

    pub async fn member_bootstrap_ready<A: ToSocketAddrs>(
        leader_addr: A,
        node_id: u64,
    ) -> Result<()> {
        let mut leader_client = loop {
            match create_client(&leader_addr).await {
                Ok(client) => break client,
                Err(e) => {
                    eprintln!("Leader connection failed. Cause: {}", e);
                    sleep(Duration::from_secs(1)).await;
                    tokio::task::yield_now().await;
                    continue;
                }
            }
        };

        let response = leader_client
            .member_bootstrap_ready(Request::new(MemberBootstrapReadyArgs { node_id }))
            .await?
            .into_inner();

        match response.code() {
            ResultCode::Ok => {
                println!("Member send the bootstrap ready request successfully.");
            }
            ResultCode::Error => {
                eprintln!("Failed to send the bootstrap ready request.");
            }
            ResultCode::WrongLeader => {
                eprintln!("Wrong leader address. Check leader changes while sending bootstrap ready request.");
            }
        }

        Ok(())
    }

    pub async fn snapshot(&mut self) -> Result<()> {
        let store = self.raft_node.storage().await;
        let hard_state = store.hard_state()?;

        self.raft_node
            .make_snapshot(store.last_index()?, hard_state.term)
            .await;
        Ok(())
    }

    pub async fn cluster_size(&self) -> usize {
        self.raft_node.get_cluster_size().await
    }

    pub async fn join(&self, ticket: ClusterJoinTicket) {
        self.raft_node.join_cluster(ticket).await;
    }
}
