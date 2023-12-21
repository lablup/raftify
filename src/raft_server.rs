use std::net::{SocketAddr, ToSocketAddrs};
use std::time::Duration;

use crate::raft_service::raft_service_server::{RaftService, RaftServiceServer};
use crate::raft_service::{self, Empty};
use crate::request_message::ServerRequestMsg;
use crate::response_message::{ServerResponseMsg, ServerResponseResult};
use crate::Config;

use bincode::serialize;
use raft::eraftpb::{ConfChangeV2, Message as RaftMessage};
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use tokio::time::timeout;
use tonic::transport::Server;
use tonic::{Request, Response, Status};

#[derive(Clone)]
pub struct RaftServer {
    snd: mpsc::Sender<ServerRequestMsg>,
    addr: SocketAddr,
    config: Config,
    logger: slog::Logger,
}

impl RaftServer {
    pub fn new<A: ToSocketAddrs>(
        snd: mpsc::Sender<ServerRequestMsg>,
        addr: A,
        config: Config,
        logger: slog::Logger,
    ) -> Self {
        let addr = addr.to_socket_addrs().unwrap().next().unwrap();
        RaftServer {
            snd,
            addr,
            config,
            logger,
        }
    }

    pub async fn run(self) {
        let addr = self.addr;
        let logger = self.logger.clone();
        slog::info!(
            logger,
            "RaftServer starts to listen gRPC requests on \"{}\"...",
            addr
        );
        let svc = RaftServiceServer::new(self);
        Server::builder()
            .add_service(svc)
            .serve(addr)
            .await
            .expect("error running server");

        slog::debug!(logger, "RaftServer quits to listen gRPC requests.");
    }
}

#[tonic::async_trait]
impl RaftService for RaftServer {
    async fn request_id(
        &self,
        request: Request<Empty>,
    ) -> Result<Response<raft_service::RequestIdResponse>, Status> {
        let _request_args = request.into_inner();
        let sender = self.snd.clone();
        let (tx, rx) = oneshot::channel();
        sender
            .send(ServerRequestMsg::RequestId { chan: tx })
            .await
            .unwrap();
        let response = rx.await.unwrap();

        match response {
            ServerResponseMsg::RequestId {
                result,
                reserved_id,
                leader_id,
                leader_addr,
                peers,
            } => match result {
                ServerResponseResult::Success => {
                    Ok(Response::new(raft_service::RequestIdResponse {
                        code: raft_service::ResultCode::Ok as i32,
                        leader_id: leader_id.unwrap(),
                        leader_addr: self.addr.to_string(),
                        reserved_id: reserved_id.unwrap(),
                        peers: serialize(&peers.unwrap()).unwrap(),
                    }))
                }
                ServerResponseResult::WrongLeader {
                    leader_id,
                    leader_addr,
                } => Ok(Response::new(raft_service::RequestIdResponse {
                    code: raft_service::ResultCode::WrongLeader as i32,
                    leader_id,
                    leader_addr,
                    reserved_id: 0,
                    peers: vec![],
                })),
                _ => unreachable!(),
            },
            _ => unreachable!(),
        }
    }

    async fn change_config(
        &self,
        request: Request<ConfChangeV2>,
    ) -> Result<Response<raft_service::ChangeConfigResponse>, Status> {
        let request_args = request.into_inner();
        let sender = self.snd.clone();
        let (tx, rx) = oneshot::channel();

        let message = ServerRequestMsg::ConfigChange {
            conf_change: request_args,
            chan: tx,
        };

        // TODO: Handle this kind of errors
        match sender.send(message).await {
            Ok(_) => (),
            Err(_) => slog::error!(self.logger, "send error"),
        }

        let mut reply = raft_service::ChangeConfigResponse::default();

        match timeout(
            Duration::from_secs_f32(self.config.conf_change_request_timeout),
            rx,
        )
        .await
        {
            Ok(Ok(_raft_response)) => {
                reply.result_type =
                    raft_service::ChangeConfigResultType::ChangeConfigSuccess as i32;
                reply.data = vec![];
            }
            Ok(_) => (),
            Err(_e) => {
                reply.result_type =
                    raft_service::ChangeConfigResultType::ChangeConfigTimeoutError as i32;
                reply.data = vec![];
                slog::error!(self.logger, "timeout waiting for reply");
            }
        }

        Ok(Response::new(reply))
    }

    async fn send_message(
        &self,
        request: Request<RaftMessage>,
    ) -> Result<Response<raft_service::Empty>, Status> {
        let request_args = request.into_inner();
        let sender = self.snd.clone();
        match sender
            .send(ServerRequestMsg::RaftMessage {
                message: Box::new(request_args),
            })
            .await
        {
            Ok(_) => (),
            Err(_) => slog::error!(self.logger, "send error"),
        }

        Ok(Response::new(raft_service::Empty {}))
    }

    async fn debug_node(
        &self,
        request: Request<Empty>,
    ) -> Result<Response<raft_service::DebugNodeResponse>, Status> {
        let _request_args = request.into_inner();
        let sender = self.snd.clone();
        let (tx, rx) = oneshot::channel();

        match sender.send(ServerRequestMsg::DebugNode { chan: tx }).await {
            Ok(_) => (),
            Err(_) => slog::error!(self.logger, "send error"),
        }

        let response = rx.await.unwrap();
        match response {
            ServerResponseMsg::DebugNode { result } => {
                Ok(Response::new(raft_service::DebugNodeResponse { result }))
            }
            _ => unreachable!(),
        }
    }

    async fn member_bootstrap_ready(
        &self,
        request: Request<raft_service::MemberBootstrapReadyArgs>,
    ) -> Result<Response<raft_service::MemberBootstrapReadyResponse>, Status> {
        let request_args = request.into_inner();
        let (tx, rx) = oneshot::channel();
        let sender = self.snd.clone();
        match sender
            .send(ServerRequestMsg::MemberBootstrapReady {
                node_id: request_args.node_id,
                chan: tx,
            })
            .await
        {
            Ok(_) => (),
            Err(_) => slog::error!(self.logger, "send error"),
        }
        let _response = rx.await.unwrap();
        Ok(Response::new(raft_service::MemberBootstrapReadyResponse {
            code: raft_service::ResultCode::Ok as i32,
        }))
    }

    async fn cluster_bootstrap_ready(
        &self,
        request: Request<raft_service::ClusterBootstrapReadyArgs>,
    ) -> Result<Response<raft_service::ClusterBootstrapReadyResponse>, Status> {
        let _request_args = request.into_inner();
        let (tx, rx) = oneshot::channel();
        let sender = self.snd.clone();
        match sender
            .send(ServerRequestMsg::ClusterBootstrapReady { chan: tx })
            .await
        {
            Ok(_) => (),
            Err(_) => slog::error!(self.logger, "send error"),
        }
        let _response = rx.await.unwrap();
        Ok(Response::new(raft_service::ClusterBootstrapReadyResponse {
            code: raft_service::ResultCode::Ok as i32,
        }))
    }
}
