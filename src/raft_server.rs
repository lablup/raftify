use std::net::{SocketAddr, ToSocketAddrs};
use std::time::Duration;

use crate::raft_service::raft_service_server::{RaftService, RaftServiceServer};
use crate::raft_service::{self, Empty, RequestIdArgs};
use crate::request_message::RequestMessage;
use crate::response_message::ResponseMessage;
use crate::Peers;

use bincode::{deserialize, serialize};
use raft::eraftpb::{ConfChangeV2, Message as RaftMessage};
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use tokio::time::timeout;
use tonic::transport::Server;
use tonic::{Request, Response, Status};

#[derive(Clone)]
pub struct RaftServer {
    snd: mpsc::Sender<RequestMessage>,
    addr: SocketAddr,
    logger: slog::Logger,
}

impl RaftServer {
    pub fn new<A: ToSocketAddrs>(
        snd: mpsc::Sender<RequestMessage>,
        addr: A,
        logger: slog::Logger,
    ) -> Self {
        let addr = addr.to_socket_addrs().unwrap().next().unwrap();
        RaftServer { snd, addr, logger }
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
        request: Request<RequestIdArgs>,
    ) -> Result<Response<raft_service::RequestIdResponse>, Status> {
        let _request_args = request.into_inner();
        let sender = self.snd.clone();
        let (tx, rx) = oneshot::channel();
        let _ = sender.send(RequestMessage::RequestId { chan: tx }).await;
        let response = rx.await.unwrap();
        match response {
            ResponseMessage::WrongLeader {
                leader_id,
                leader_addr,
            } => {
                slog::warn!(self.logger, "sending wrong leader");
                Ok(Response::new(raft_service::RequestIdResponse {
                    code: raft_service::ResultCode::WrongLeader as i32,
                    leader_id,
                    leader_addr,
                    reserved_id: 0,
                    peers: vec![],
                }))
            }
            ResponseMessage::IdReserved {
                reserved_id,
                leader_id,
                peers,
            } => Ok(Response::new(raft_service::RequestIdResponse {
                code: raft_service::ResultCode::Ok as i32,
                leader_id,
                leader_addr: self.addr.to_string(),
                reserved_id,
                peers: serialize(&peers).unwrap(),
            })),
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

        let message = RequestMessage::ConfigChange {
            conf_change: request_args,
            chan: tx,
        };

        // TODO: Handle this kind of errors
        match sender.send(message).await {
            Ok(_) => (),
            Err(_) => slog::error!(self.logger, "send error"),
        }

        let mut reply = raft_service::ChangeConfigResponse::default();
        match timeout(Duration::from_secs(2), rx).await {
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
                log::error!("timeout waiting for reply");
            }
        }

        Ok(Response::new(reply))
    }

    async fn send_message(
        &self,
        request: Request<RaftMessage>,
    ) -> Result<Response<raft_service::RaftResponse>, Status> {
        let request_args = request.into_inner();
        let sender = self.snd.clone();
        match sender
            .send(RequestMessage::RaftMessage {
                message: Box::new(request_args),
            })
            .await
        {
            Ok(_) => (),
            Err(_) => slog::error!(self.logger, "send error"),
        }

        let response = ResponseMessage::Ok;
        Ok(Response::new(raft_service::RaftResponse {
            inner: serialize(&response).unwrap(),
        }))
    }

    async fn debug_node(
        &self,
        request: Request<Empty>,
    ) -> Result<Response<raft_service::DebugNodeResponse>, Status> {
        let _request_args = request.into_inner();
        let sender = self.snd.clone();
        let (tx, rx) = oneshot::channel();

        match sender.send(RequestMessage::DebugNode { chan: tx }).await {
            Ok(_) => (),
            Err(_) => slog::error!(self.logger, "send error"),
        }

        let response = rx.await.unwrap();
        match response {
            ResponseMessage::DebugNode { result } => {
                Ok(Response::new(raft_service::DebugNodeResponse { result }))
            }
            _ => unreachable!(),
        }
    }
}
