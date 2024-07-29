use bincode::serialize;
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
    raft_service::{self},
    Config, Error,
};
use crate::{
    create_client,
    raft::{eraftpb::Message as RaftMessage, logger::Logger},
    raft_service::{
        raft_service_server::{RaftService, RaftServiceServer},
        ProposeArgs,
    },
    request::{
        common::confchange_request::ConfChangeRequest, server_request_message::ServerRequestMsg,
    },
    response::server_response_message::{
        ConfChangeResponseResult, RequestIdResponseResult, ResponseResult, ServerResponseMsg,
    },
    AbstractLogEntry, AbstractStateMachine,
};

#[cfg(feature = "inspection_api")]
use crate::raft_service::inspection_service::raft_inspection_service_server::RaftInspectionService;

#[cfg(feature = "manipulation_api")]
use crate::raft_service::manipulation_service::raft_manipulation_service_server::RaftManipulationService;

#[derive(Clone)]
pub struct RaftServer<LogEntry: AbstractLogEntry, FSM: AbstractStateMachine> {
    tx: mpsc::Sender<ServerRequestMsg<LogEntry, FSM>>,
    raft_addr: SocketAddr,
    config: Config,
    logger: Arc<dyn Logger>,
}

impl<LogEntry: AbstractLogEntry + 'static, FSM: AbstractStateMachine + 'static>
    RaftServer<LogEntry, FSM>
{
    pub fn new<A: ToSocketAddrs>(
        tx: mpsc::Sender<ServerRequestMsg<LogEntry, FSM>>,
        raft_addr: A,
        config: Config,
        logger: Arc<dyn Logger>,
    ) -> Self {
        let raft_addr = raft_addr.to_socket_addrs().unwrap().next().unwrap();
        RaftServer {
            tx,
            raft_addr,
            config,
            logger,
        }
    }

    pub(crate) async fn run(self, rx_quit_signal: Receiver<()>) -> Result<(), Error> {
        let raft_addr = self.raft_addr;
        let logger = self.logger.clone();
        logger.debug(&format!(
            "RaftServer starts to listen gRPC requests on \"{}\"...",
            raft_addr
        ));

        let quit_signal = async {
            rx_quit_signal.await.ok();
        };

        Server::builder()
            .add_service(RaftServiceServer::new(self))
            .serve_with_shutdown(raft_addr, quit_signal)
            .await?;

        Ok(())
    }
}

impl<LogEntry: AbstractLogEntry + 'static, FSM: AbstractStateMachine + 'static>
    RaftServer<LogEntry, FSM>
{
    fn print_send_error(&self, function_name: &str) {
        self.logger.error(&format!(
            "Error occurred in sending message ('RaftServer --> RaftNode'). Function: '{}'",
            function_name
        ));
    }
}

#[tonic::async_trait]
impl<LogEntry: AbstractLogEntry + 'static, FSM: AbstractStateMachine + 'static> RaftService
    for RaftServer<LogEntry, FSM>
{
    async fn request_id(
        &self,
        request: Request<raft_service::RequestIdArgs>,
    ) -> Result<Response<raft_service::RequestIdResponse>, Status> {
        let request_args = request.into_inner();
        let sender = self.tx.clone();
        let (tx_msg, rx_msg) = oneshot::channel();
        sender
            .send(ServerRequestMsg::RequestId {
                raft_addr: request_args.raft_addr.clone(),
                tx_msg,
            })
            .await
            .unwrap();
        let response = rx_msg.await.unwrap();

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
                    leader_addr: self.raft_addr.to_string(),
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
                RequestIdResponseResult::WrongLeader { leader_addr, .. } => {
                    let mut client = create_client(leader_addr).await.unwrap();
                    let reply = client.request_id(request_args).await?.into_inner();

                    Ok(Response::new(reply))
                }
            },
            _ => unreachable!(),
        }
    }

    async fn change_config(
        &self,
        request: Request<raft_service::ChangeConfigArgs>,
    ) -> Result<Response<raft_service::ChangeConfigResponse>, Status> {
        let request_args = request.into_inner();
        let sender = self.tx.clone();
        let (tx_msg, rx_msg) = oneshot::channel();

        let conf_change_request: ConfChangeRequest = request_args.clone().into();

        let message = ServerRequestMsg::ChangeConfig {
            conf_change: conf_change_request,
            tx_msg,
        };

        // TODO: Handle this kind of errors
        match sender.send(message).await {
            Ok(_) => {}
            Err(_) => {
                self.print_send_error(function_name!());
            }
        }

        let mut reply = raft_service::ChangeConfigResponse::default();

        match timeout(
            Duration::from_secs_f32(self.config.conf_change_request_timeout),
            rx_msg,
        )
        .await
        {
            Ok(Ok(raft_response)) => {
                match raft_response {
                    ServerResponseMsg::ConfigChange { result } => match result {
                        ConfChangeResponseResult::JoinSuccess {
                            assigned_ids,
                            peers,
                        } => {
                            reply.result_type =
                                raft_service::ChangeConfigResultType::ChangeConfigSuccess as i32;
                            reply.assigned_ids = assigned_ids;
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
                        ConfChangeResponseResult::WrongLeader { leader_addr, .. } => {
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
                self.logger.error(&format!(
                    "Confchange request timeout! (\"conf_change_request_timeout\" = {})",
                    self.config.conf_change_request_timeout
                ));
            }
        }

        Ok(Response::new(reply))
    }

    async fn send_message(
        &self,
        request: Request<RaftMessage>,
    ) -> Result<Response<raft_service::Empty>, Status> {
        let request_args = request.into_inner();
        let sender = self.tx.clone();
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
    ) -> Result<Response<raft_service::ProposeResponse>, Status> {
        let request_args = request.into_inner();
        let sender = self.tx.clone();

        let (tx_msg, rx_msg) = oneshot::channel();
        match sender
            .send(ServerRequestMsg::Propose {
                proposal: request_args.msg.clone(),
                tx_msg,
            })
            .await
        {
            Ok(_) => (),
            Err(_) => self.print_send_error(function_name!()),
        }

        let response = rx_msg.await.unwrap();
        match response {
            ServerResponseMsg::Propose { result } => {
                match result {
                    ResponseResult::Success => Ok(Response::new(raft_service::ProposeResponse {
                        ..Default::default()
                    })),
                    ResponseResult::Error(error) => {
                        Ok(Response::new(raft_service::ProposeResponse {
                            error: error.to_string().as_bytes().to_vec(),
                        }))
                    }
                    ResponseResult::WrongLeader { leader_addr, .. } => {
                        // TODO: Handle this kind of errors
                        let mut client = create_client(leader_addr).await.unwrap();
                        let _ = client
                            .propose(ProposeArgs {
                                msg: request_args.msg,
                            })
                            .await?;

                        Ok(Response::new(raft_service::ProposeResponse {
                            ..Default::default()
                        }))
                    }
                }
            }
            _ => unreachable!(),
        }
    }

    async fn debug_node(
        &self,
        request: Request<raft_service::Empty>,
    ) -> Result<Response<raft_service::DebugNodeResponse>, Status> {
        let _request_args = request.into_inner();
        let sender = self.tx.clone();
        let (tx_msg, rx_msg) = oneshot::channel();

        match sender.send(ServerRequestMsg::DebugNode { tx_msg }).await {
            Ok(_) => (),
            Err(_) => self.print_send_error(function_name!()),
        }

        let response = rx_msg.await.unwrap();
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
        let (tx_msg, rx_msg) = oneshot::channel();
        let sender = self.tx.clone();
        match sender.send(ServerRequestMsg::GetPeers { tx_msg }).await {
            Ok(_) => (),
            Err(_) => self.print_send_error(function_name!()),
        }
        let response = rx_msg.await.unwrap();

        match response {
            ServerResponseMsg::GetPeers { peers } => {
                Ok(Response::new(raft_service::GetPeersResponse {
                    peers_json: peers.to_json(),
                }))
            }
            _ => unreachable!(),
        }
    }

    async fn leave_joint(
        &self,
        request: Request<raft_service::Empty>,
    ) -> Result<Response<raft_service::Empty>, Status> {
        let _request_args = request.into_inner();
        let (tx_msg, rx_msg) = oneshot::channel();
        let sender = self.tx.clone();
        match sender.send(ServerRequestMsg::LeaveJoint { tx_msg }).await {
            Ok(_) => (),
            Err(_) => self.print_send_error(function_name!()),
        }
        let response = rx_msg.await.unwrap();

        match response {
            ServerResponseMsg::LeaveJoint {} => Ok(Response::new(raft_service::Empty {})),
            _ => unreachable!(),
        }
    }

    async fn set_peers(
        &self,
        request: Request<raft_service::Peers>,
    ) -> Result<Response<raft_service::Empty>, Status> {
        let request_args = request.into_inner();
        let peers = request_args.into();

        let (tx_msg, rx_msg) = oneshot::channel();
        let sender = self.tx.clone();
        match sender
            .send(ServerRequestMsg::SetPeers { peers, tx_msg })
            .await
        {
            Ok(_) => (),
            Err(_) => self.print_send_error(function_name!()),
        }
        let response = rx_msg.await.unwrap();

        match response {
            ServerResponseMsg::SetPeers {} => Ok(Response::new(raft_service::Empty {})),
            _ => unreachable!(),
        }
    }

    async fn create_snapshot(
        &self,
        request: Request<raft_service::Empty>,
    ) -> Result<Response<raft_service::Empty>, Status> {
        let _request_args = request.into_inner();
        let (tx_msg, rx_msg) = oneshot::channel();
        let sender = self.tx.clone();
        match sender
            .send(ServerRequestMsg::CreateSnapshot { tx_msg })
            .await
        {
            Ok(_) => (),
            Err(_) => self.print_send_error(function_name!()),
        }
        let response = rx_msg.await.unwrap();

        match response {
            ServerResponseMsg::CreateSnapshot {} => Ok(Response::new(raft_service::Empty {})),
            _ => unreachable!(),
        }
    }
}

#[cfg(feature = "manipulation_api")]
#[tonic::async_trait]
impl<LogEntry: AbstractLogEntry + 'static, FSM: AbstractStateMachine + 'static>
    RaftManipulationService for RaftServer<LogEntry, FSM>
{
    async fn become_follower(
        &self,
        request: tonic::Request<raft_service::manipulation_service::Empty>,
    ) -> std::result::Result<
        tonic::Response<raft_service::manipulation_service::Empty>,
        tonic::Status,
    > {
        todo!()
    }

    async fn become_leader(
        &self,
        request: tonic::Request<raft_service::manipulation_service::Empty>,
    ) -> std::result::Result<
        tonic::Response<raft_service::manipulation_service::Empty>,
        tonic::Status,
    > {
        todo!()
    }

    async fn become_candidate(
        &self,
        request: tonic::Request<raft_service::manipulation_service::Empty>,
    ) -> std::result::Result<
        tonic::Response<raft_service::manipulation_service::Empty>,
        tonic::Status,
    > {
        todo!()
    }

    async fn become_pre_candidate(
        &self,
        request: tonic::Request<raft_service::manipulation_service::Empty>,
    ) -> std::result::Result<
        tonic::Response<raft_service::manipulation_service::Empty>,
        tonic::Status,
    > {
        todo!()
    }

    async fn request_snapshot(
        &self,
        request: tonic::Request<raft_service::manipulation_service::Empty>,
    ) -> std::result::Result<
        tonic::Response<raft_service::manipulation_service::Empty>,
        tonic::Status,
    > {
        todo!()
    }

    async fn create_snapshot(
        &self,
        request: tonic::Request<raft_service::manipulation_service::Empty>,
    ) -> std::result::Result<
        tonic::Response<raft_service::manipulation_service::Empty>,
        tonic::Status,
    > {
        todo!()
    }

    async fn is_promotable(
        &self,
        request: tonic::Request<raft_service::manipulation_service::Empty>,
    ) -> std::result::Result<
        tonic::Response<raft_service::manipulation_service::Empty>,
        tonic::Status,
    > {
        todo!()
    }

    async fn check_quorum_active(
        &self,
        request: tonic::Request<raft_service::manipulation_service::Empty>,
    ) -> std::result::Result<
        tonic::Response<raft_service::manipulation_service::Empty>,
        tonic::Status,
    > {
        todo!()
    }

    async fn send_timeout_now(
        &self,
        request: tonic::Request<raft_service::manipulation_service::Empty>,
    ) -> std::result::Result<
        tonic::Response<raft_service::manipulation_service::Empty>,
        tonic::Status,
    > {
        todo!()
    }

    async fn reset(
        &self,
        request: tonic::Request<raft_service::manipulation_service::Empty>,
    ) -> std::result::Result<
        tonic::Response<raft_service::manipulation_service::Empty>,
        tonic::Status,
    > {
        todo!()
    }

    async fn campaign(
        &self,
        request: Request<raft_service::manipulation_service::Empty>,
    ) -> Result<Response<raft_service::manipulation_service::Empty>, Status> {
        todo!()
    }

    async fn transfer_leader(
        &self,
        request: Request<raft_service::manipulation_service::Empty>,
    ) -> Result<Response<raft_service::manipulation_service::Empty>, Status> {
        todo!()
    }

    async fn abort_transfer_leader(
        &self,
        request: Request<raft_service::manipulation_service::Empty>,
    ) -> Result<Response<raft_service::manipulation_service::Empty>, Status> {
        todo!()
    }

    async fn ping(
        &self,
        request: Request<raft_service::manipulation_service::Empty>,
    ) -> Result<Response<raft_service::manipulation_service::Empty>, Status> {
        todo!()
    }
}

#[cfg(feature = "inspection_api")]
#[tonic::async_trait]
impl<LogEntry: AbstractLogEntry + 'static, FSM: AbstractStateMachine + 'static>
    RaftInspectionService for RaftServer<LogEntry, FSM>
{
    async fn get_pending_conf_change(
        &self,
        request: tonic::Request<raft_service::inspection_service::Empty>,
    ) -> std::result::Result<tonic::Response<raft_service::inspection_service::Empty>, tonic::Status>
    {
        todo!()
    }

    async fn get_snapshot_report(
        &self,
        request: tonic::Request<raft_service::inspection_service::Empty>,
    ) -> std::result::Result<tonic::Response<raft_service::inspection_service::Empty>, tonic::Status>
    {
        todo!()
    }

    async fn get_peers(
        &self,
        request: tonic::Request<raft_service::inspection_service::Empty>,
    ) -> std::result::Result<tonic::Response<raft_service::inspection_service::Empty>, tonic::Status>
    {
        todo!()
    }

    async fn get_progress_matched(
        &self,
        request: tonic::Request<raft_service::inspection_service::Empty>,
    ) -> std::result::Result<tonic::Response<raft_service::inspection_service::Empty>, tonic::Status>
    {
        todo!()
    }

    async fn get_progress_next_idx(
        &self,
        request: tonic::Request<raft_service::inspection_service::Empty>,
    ) -> std::result::Result<tonic::Response<raft_service::inspection_service::Empty>, tonic::Status>
    {
        todo!()
    }

    async fn get_progress_paused(
        &self,
        request: tonic::Request<raft_service::inspection_service::Empty>,
    ) -> std::result::Result<tonic::Response<raft_service::inspection_service::Empty>, tonic::Status>
    {
        todo!()
    }

    async fn get_progress_pending_snapshot(
        &self,
        request: tonic::Request<raft_service::inspection_service::Empty>,
    ) -> std::result::Result<tonic::Response<raft_service::inspection_service::Empty>, tonic::Status>
    {
        todo!()
    }

    async fn get_progress_pending_request_snapshot(
        &self,
        request: tonic::Request<raft_service::inspection_service::Empty>,
    ) -> std::result::Result<tonic::Response<raft_service::inspection_service::Empty>, tonic::Status>
    {
        todo!()
    }

    async fn get_progress_recent_active(
        &self,
        request: tonic::Request<raft_service::inspection_service::Empty>,
    ) -> std::result::Result<tonic::Response<raft_service::inspection_service::Empty>, tonic::Status>
    {
        todo!()
    }

    async fn get_progress_commit_group_id(
        &self,
        request: tonic::Request<raft_service::inspection_service::Empty>,
    ) -> std::result::Result<tonic::Response<raft_service::inspection_service::Empty>, tonic::Status>
    {
        todo!()
    }

    // Implement the remaining missing trait items here
    async fn get_progress_inflights(
        &self,
        request: tonic::Request<raft_service::inspection_service::Empty>,
    ) -> std::result::Result<tonic::Response<raft_service::inspection_service::Empty>, tonic::Status>
    {
        todo!()
    }

    async fn get_progress_state(
        &self,
        request: tonic::Request<raft_service::inspection_service::Empty>,
    ) -> std::result::Result<tonic::Response<raft_service::inspection_service::Empty>, tonic::Status>
    {
        todo!()
    }

    async fn get_node_id(
        &self,
        request: tonic::Request<raft_service::inspection_service::Empty>,
    ) -> std::result::Result<tonic::Response<raft_service::inspection_service::Empty>, tonic::Status>
    {
        todo!()
    }

    async fn get_leader_id(
        &self,
        request: tonic::Request<raft_service::inspection_service::Empty>,
    ) -> std::result::Result<tonic::Response<raft_service::inspection_service::Empty>, tonic::Status>
    {
        todo!()
    }

    async fn get_term(
        &self,
        request: tonic::Request<raft_service::inspection_service::Empty>,
    ) -> std::result::Result<tonic::Response<raft_service::inspection_service::Empty>, tonic::Status>
    {
        todo!()
    }

    async fn get_hard_state_term(
        &self,
        request: tonic::Request<raft_service::inspection_service::Empty>,
    ) -> std::result::Result<tonic::Response<raft_service::inspection_service::Empty>, tonic::Status>
    {
        todo!()
    }

    async fn get_hard_state_vote(
        &self,
        request: tonic::Request<raft_service::inspection_service::Empty>,
    ) -> std::result::Result<tonic::Response<raft_service::inspection_service::Empty>, tonic::Status>
    {
        todo!()
    }

    async fn get_hard_state_commit(
        &self,
        request: tonic::Request<raft_service::inspection_service::Empty>,
    ) -> std::result::Result<tonic::Response<raft_service::inspection_service::Empty>, tonic::Status>
    {
        todo!()
    }

    async fn get_conf_state_voters(
        &self,
        request: tonic::Request<raft_service::inspection_service::Empty>,
    ) -> std::result::Result<tonic::Response<raft_service::inspection_service::Empty>, tonic::Status>
    {
        todo!()
    }

    async fn get_conf_state_learners(
        &self,
        request: tonic::Request<raft_service::inspection_service::Empty>,
    ) -> std::result::Result<tonic::Response<raft_service::inspection_service::Empty>, tonic::Status>
    {
        todo!()
    }

    async fn get_conf_state_voters_outgoing(
        &self,
        request: tonic::Request<raft_service::inspection_service::Empty>,
    ) -> std::result::Result<tonic::Response<raft_service::inspection_service::Empty>, tonic::Status>
    {
        todo!()
    }

    async fn get_conf_state_learners_next(
        &self,
        request: tonic::Request<raft_service::inspection_service::Empty>,
    ) -> std::result::Result<tonic::Response<raft_service::inspection_service::Empty>, tonic::Status>
    {
        todo!()
    }

    async fn get_conf_state_snapshot(
        &self,
        request: tonic::Request<raft_service::inspection_service::Empty>,
    ) -> std::result::Result<tonic::Response<raft_service::inspection_service::Empty>, tonic::Status>
    {
        todo!()
    }

    async fn get_conf_state_last_index(
        &self,
        request: tonic::Request<raft_service::inspection_service::Empty>,
    ) -> std::result::Result<tonic::Response<raft_service::inspection_service::Empty>, tonic::Status>
    {
        todo!()
    }

    async fn get_raft_log_committed(
        &self,
        request: tonic::Request<raft_service::inspection_service::Empty>,
    ) -> std::result::Result<tonic::Response<raft_service::inspection_service::Empty>, tonic::Status>
    {
        todo!()
    }

    async fn get_raft_log_applied(
        &self,
        request: tonic::Request<raft_service::inspection_service::Empty>,
    ) -> std::result::Result<tonic::Response<raft_service::inspection_service::Empty>, tonic::Status>
    {
        todo!()
    }

    async fn get_raft_log_persisted(
        &self,
        request: tonic::Request<raft_service::inspection_service::Empty>,
    ) -> std::result::Result<tonic::Response<raft_service::inspection_service::Empty>, tonic::Status>
    {
        todo!()
    }
}
