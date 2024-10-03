use crate::{
    config::TlsConfig, raft::logger::Logger, request::server_request_message::ServerRequestMsg,
    ClusterJoinTicket, InitialRole, Peers, StableStorage,
};
use bincode::deserialize;
use std::{net::ToSocketAddrs, ops::Deref, sync::Arc};
use tokio::{
    signal,
    sync::{mpsc, oneshot},
};

use super::{
    create_client,
    error::{Error, Result},
    raft_node::RaftNode,
    raft_server::RaftServer,
    raft_service::{self, ResultCode},
    AbstractLogEntry, AbstractStateMachine, Config,
};

/// This structure contains functions required for Raft bootstrap along with RaftNode and RaftServer instances.
/// The bootstrap function returns an instance of the Raft type that deref to RaftNode type,
/// allowing the use of functions necessary for interaction with the cluster.
#[derive(Clone)]
pub struct Raft<
    LogEntry: AbstractLogEntry + 'static,
    LogStorage: StableStorage + Send + Clone + 'static,
    FSM: AbstractStateMachine + Clone + 'static,
> {
    pub raft_node: RaftNode<LogEntry, LogStorage, FSM>,
    pub raft_server: RaftServer<LogEntry, LogStorage, FSM>,
    pub tx_server: mpsc::Sender<ServerRequestMsg<LogEntry, LogStorage, FSM>>,
    pub logger: Arc<dyn Logger>,
}

impl<
        LogEntry: AbstractLogEntry + 'static,
        LogStorage: StableStorage + Send + Clone + 'static,
        FSM: AbstractStateMachine + Clone + 'static,
    > Deref for Raft<LogEntry, LogStorage, FSM>
{
    type Target = RaftNode<LogEntry, LogStorage, FSM>;

    fn deref(&self) -> &Self::Target {
        &self.raft_node
    }
}

impl<
        LogEntry: AbstractLogEntry,
        LogStorage: StableStorage + Send + Sync + Clone + 'static,
        FSM: AbstractStateMachine + Send + Sync + Clone + 'static,
    > Raft<LogEntry, LogStorage, FSM>
{
    /// Creates a new Raft instance.
    /// Cloning a Raft instance does not bootstrap a new Raft instance.
    /// To bootstrap a new Raft instance, call this associated function.
    pub fn bootstrap<A: ToSocketAddrs>(
        node_id: u64,
        raft_addr: A,
        log_storage: LogStorage,
        fsm: FSM,
        config: Config,
        logger: Arc<dyn Logger>,
    ) -> Result<Self> {
        logger.info(&format!("RaftNode bootstrapped. {:?}", config));

        let raft_addr = raft_addr.to_socket_addrs()?.next().unwrap();
        let mut should_be_leader = config.initial_peers.is_none();

        if config.initial_peers.is_some() {
            let leaders = config
                .initial_peers
                .clone()
                .unwrap()
                .inner
                .into_iter()
                .filter(|(_, peer)| peer.initial_role == InitialRole::Leader)
                .map(|(key, _)| key)
                .collect::<Vec<_>>();

            assert!(leaders.len() < 2);
            should_be_leader = leaders.contains(&node_id);
        }

        let (tx_server, rx_server) = mpsc::channel(100);
        let raft_node = RaftNode::bootstrap(
            node_id,
            should_be_leader,
            log_storage,
            fsm,
            config.clone(),
            raft_addr,
            logger.clone(),
            tx_server.clone(),
            rx_server,
        )?;

        let raft_server =
            RaftServer::new(tx_server.clone(), raft_addr, config.clone(), logger.clone());

        Ok(Self {
            tx_server: tx_server.clone(),
            raft_node,
            raft_server,
            logger,
        })
    }

    /// Starts the RaftNode and RaftServer.
    pub async fn run(self) -> Result<()> {
        let (tx_quit_signal, rx_quit_signal) = oneshot::channel::<()>();

        let raft_node = self.raft_node.clone();
        let raft_node_handle = tokio::spawn(raft_node.run());
        let raft_server = self.raft_server.clone();
        let raft_server_handle = tokio::spawn(raft_server.run(rx_quit_signal));

        tokio::select! {
            _ = signal::ctrl_c() => {
                self.logger.info("Ctrl+C signal detected. Shutting down...");
                Ok(())
            }
            result = raft_node_handle => {
                tx_quit_signal.send(()).unwrap();

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

    /// Requests a cluster join ticket from the peer.
    /// You can use this to dynamically add members in addition to initial_peers.
    pub async fn request_id<A: ToSocketAddrs>(
        raft_addr: A,
        peer_addr: String,
        tls_config: Option<TlsConfig>,
    ) -> Result<ClusterJoinTicket> {
        let raft_addr = raft_addr
            .to_socket_addrs()
            .unwrap()
            .next()
            .unwrap()
            .to_string();

        let mut client = create_client(&peer_addr, tls_config).await?;
        let response = client
            .request_id(raft_service::RequestIdArgs {
                raft_addr: raft_addr.to_string(),
            })
            .await?
            .into_inner();

        let peers: Peers = deserialize(&response.peers)?;
        match response.code() {
            ResultCode::Ok => Ok(ClusterJoinTicket {
                raft_addr,
                reserved_id: response.reserved_id,
                leader_addr: response.leader_addr,
                peers: peers.into(),
            }),
            ResultCode::Error => Err(Error::JoinError),
            ResultCode::WrongLeader => {
                unreachable!();
            }
        }
    }
}
