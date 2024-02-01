use bincode::serialize;
use jopemachine_raft::logger::Logger;
use std::{
    net::{SocketAddr, ToSocketAddrs},
    sync::Arc,
    time::Duration,
};
use tokio::{
    sync::{
        mpsc,
        oneshot::{self, Receiver},
    },
    time::timeout,
};
use tonic::{transport::Server, Request, Response, Status};

use super::{
    macro_utils::function_name,
    raft_service::{
        self,
        raft_service_server::{RaftService, RaftServiceServer},
        Empty,
    },
    request_message::ServerRequestMsg,
    response_message::{RequestIdResponseResult, ServerResponseMsg},
    Config, Error,
};
use crate::{
    create_client,
    raft::eraftpb::{ConfChangeV2, Message as RaftMessage},
    raft_service::ProposeArgs,
    response_message::{ConfChangeResponseResult, ResponseResult},
};

#[derive(Clone)]
pub struct RaftServer {
    snd: mpsc::Sender<ServerRequestMsg>,
    addr: SocketAddr,
    config: Config,
    logger: Arc<dyn Logger>,
}

impl RaftServer {
    pub fn new<A: ToSocketAddrs>(
        snd: mpsc::Sender<ServerRequestMsg>,
        addr: A,
        config: Config,
        logger: Arc<dyn Logger>,
    ) -> Self {
        let addr = addr.to_socket_addrs().unwrap().next().unwrap();
        RaftServer {
            snd,
            addr,
            config,
            logger,
        }
    }

    pub async fn run(self, quit_signal_rx: Receiver<()>) -> Result<(), Error> {
        let addr = self.addr;
        let logger = self.logger.clone();
        logger.debug(&format!(
            "RaftServer starts to listen gRPC requests on \"{}\"...",
            addr
        ));

        let shutdown_signal = async {
            quit_signal_rx.await.ok();
        };

        Server::builder()
            .add_service(RaftServiceServer::new(self))
            .serve_with_shutdown(addr, shutdown_signal)
            .await?;

        Ok(())
    }
}

impl RaftServer {
    fn print_send_error(&self, function_name: &str) {
        self.logger.error(&format!(
            "Error occurred in sending message ('RaftServer --> RaftNode'). Function: '{}'",
            function_name
        ));
    }
}

#[tonic::async_trait]
impl RaftService for RaftServer {
    async fn request_id(
        &self,
        request: Request<raft_service::RequestIdArgs>,
    ) -> Result<Response<raft_service::RequestIdResponse>, Status> {
        let request_args = request.into_inner();
        let sender = self.snd.clone();
        let (tx, rx) = oneshot::channel();
        sender
            .send(ServerRequestMsg::RequestId {
                raft_addr: request_args.raft_addr.clone(),
                chan: tx,
            })
            .await
            .unwrap();
        let response = rx.await.unwrap();

        match response {
            ServerResponseMsg::RequestId { result } => match result {
                RequestIdResponseResult::Success {
                    reserved_id,
                    leader_id,
                    peers,
                } => Ok(Response::new(raft_service::RequestIdResponse {
                    code: raft_service::ResultCode::Ok as i32,
                    leader_id,
                    reserved_id,
                    leader_addr: self.addr.to_string(),
                    peers: serialize(&peers).unwrap(),
                    ..Default::default()
                })),
                RequestIdResponseResult::Error(e) => {
                    Ok(Response::new(raft_service::RequestIdResponse {
                        code: raft_service::ResultCode::Error as i32,
                        error: e.to_string().as_bytes().to_vec(),
                        ..Default::default()
                    }))
                }
                RequestIdResponseResult::WrongLeader {
                    leader_addr,
                    ..
                } => {
                    let mut client = create_client(leader_addr).await.unwrap();
                    let reply = client.request_id(request_args).await?.into_inner();

                    Ok(Response::new(reply))
                },
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

        let message = ServerRequestMsg::ChangeConfig {
            conf_change: request_args.clone(),
            chan: tx,
        };

        // TODO: Handle this kind of errors
        match sender.send(message).await {
            Ok(_) => (),
            Err(_) => self.print_send_error(function_name!()),
        }

        let mut reply = raft_service::ChangeConfigResponse::default();

        match timeout(
            Duration::from_secs_f32(self.config.conf_change_request_timeout),
            rx,
        )
        .await
        {
            Ok(Ok(raft_response)) => {
                match raft_response {
                    ServerResponseMsg::ConfigChange { result } => match result {
                        ConfChangeResponseResult::JoinSuccess { assigned_id, peers } => {
                            reply.result_type =
                                raft_service::ChangeConfigResultType::ChangeConfigSuccess as i32;
                            reply.assigned_id = assigned_id;
                            reply.peers = serialize(&peers).unwrap();
                        }
                        ConfChangeResponseResult::RemoveSuccess {} => {
                            reply.result_type =
                                raft_service::ChangeConfigResultType::ChangeConfigSuccess as i32;
                        }
                        ConfChangeResponseResult::Error(e) => {
                            reply.result_type =
                                raft_service::ChangeConfigResultType::ChangeConfigUnknownError
                                    as i32;
                            reply.error = e.to_string().as_bytes().to_vec();
                        }
                        ConfChangeResponseResult::WrongLeader {
                            leader_addr,
                            ..
                        } => {
                            reply.result_type =
                                raft_service::ChangeConfigResultType::ChangeConfigWrongLeader
                                    as i32;

                            let mut client = create_client(leader_addr).await.unwrap();
                            reply = client.change_config(request_args).await?.into_inner();
                        }
                    },
                    _ => unreachable!(),
                }
                reply.result_type =
                    raft_service::ChangeConfigResultType::ChangeConfigSuccess as i32;
            }
            Ok(Err(e)) => {
                reply.result_type =
                    raft_service::ChangeConfigResultType::ChangeConfigUnknownError as i32;
                reply.error = e.to_string().as_bytes().to_vec();
            }
            Err(e) => {
                reply.result_type =
                    raft_service::ChangeConfigResultType::ChangeConfigTimeoutError as i32;
                reply.error = e.to_string().as_bytes().to_vec();
                self.logger.error("timeout waiting for reply");
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
            .send(ServerRequestMsg::SendMessage {
                message: Box::new(request_args),
            })
            .await
        {
            Ok(_) => (),
            Err(_) => self.print_send_error(function_name!()),
        }

        Ok(Response::new(raft_service::Empty {}))
    }

    async fn propose(
        &self,
        request: Request<raft_service::ProposeArgs>,
    ) -> Result<Response<raft_service::Empty>, Status> {
        let request_args = request.into_inner();
        let sender = self.snd.clone();

        let (tx, rx) = oneshot::channel();
        match sender
            .send(ServerRequestMsg::Propose {
                proposal: request_args.msg.clone(),
                chan: tx,
            })
            .await
        {
            Ok(_) => (),
            Err(_) => self.print_send_error(function_name!()),
        }

        let response = rx.await.unwrap();
        match response {
            ServerResponseMsg::Propose { result } => {
                match result {
                    ResponseResult::Success => Ok(Response::new(raft_service::Empty {})),
                    ResponseResult::Error(_) => Ok(Response::new(raft_service::Empty {})),
                    ResponseResult::WrongLeader {
                        leader_addr,
                        ..
                    } => {
                        // TODO: Handle this kind of errors
                        let mut client = create_client(leader_addr).await.unwrap();
                        let _ = client
                            .propose(ProposeArgs {
                                msg: request_args.msg,
                            })
                            .await?;

                        Ok(Response::new(raft_service::Empty {}))
                    }
                }
            }
            _ => unreachable!(),
        }
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
            Err(_) => self.print_send_error(function_name!()),
        }

        let response = rx.await.unwrap();
        match response {
            ServerResponseMsg::DebugNode { result_json } => {
                Ok(Response::new(raft_service::DebugNodeResponse {
                    result_json,
                }))
            }
            _ => unreachable!(),
        }
    }

    async fn get_peers(
        &self,
        request: Request<raft_service::Empty>,
    ) -> Result<Response<raft_service::GetPeersResponse>, Status> {
        let _request_args = request.into_inner();
        let (tx, rx) = oneshot::channel();
        let sender = self.snd.clone();
        match sender.send(ServerRequestMsg::GetPeers { chan: tx }).await {
            Ok(_) => (),
            Err(_) => self.print_send_error(function_name!()),
        }
        let response = rx.await.unwrap();

        match response {
            ServerResponseMsg::GetPeers { peers } => {
                Ok(Response::new(raft_service::GetPeersResponse {
                    peers_json: peers.to_json(),
                }))
            }
            _ => unreachable!(),
        }
    }

    async fn create_snapshot(
        &self,
        request: Request<raft_service::Empty>,
    ) -> Result<Response<raft_service::Empty>, Status> {
        let _request_args = request.into_inner();
        let (tx, rx) = oneshot::channel();
        let sender = self.snd.clone();
        match sender
            .send(ServerRequestMsg::CreateSnapshot { chan: tx })
            .await
        {
            Ok(_) => (),
            Err(_) => self.print_send_error(function_name!()),
        }
        let response = rx.await.unwrap();

        match response {
            ServerResponseMsg::CreateSnapshot {} => Ok(Response::new(raft_service::Empty {})),
            _ => unreachable!(),
        }
    }
}
