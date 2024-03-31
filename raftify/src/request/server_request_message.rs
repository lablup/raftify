use std::marker::PhantomData;

use tokio::sync::oneshot::Sender;

use crate::{
    raft::eraftpb::Message as RaftMessage, response::server_response_message::ServerResponseMsg,
    AbstractLogEntry, AbstractStateMachine, Peers,
};

use super::common::confchange_request::ConfChangeRequest;

/// Request type processed through network calls (gRPC)
#[derive(Debug)]
pub enum ServerRequestMsg<LogEntry: AbstractLogEntry, FSM: AbstractStateMachine> {
    RequestId {
        raft_addr: String,
        tx_msg: Sender<ServerResponseMsg>,
    },
    Propose {
        proposal: Vec<u8>,
        tx_msg: Sender<ServerResponseMsg>,
    },
    ChangeConfig {
        conf_change: ConfChangeRequest,
        tx_msg: Sender<ServerResponseMsg>,
    },
    DebugNode {
        tx_msg: Sender<ServerResponseMsg>,
    },
    SendMessage {
        message: Box<RaftMessage>,
    },
    GetPeers {
        tx_msg: Sender<ServerResponseMsg>,
    },
    SetPeers {
        peers: Peers,
        tx_msg: Sender<ServerResponseMsg>,
    },
    LeaveJoint {
        tx_msg: Sender<ServerResponseMsg>,
    },
    CreateSnapshot {
        tx_msg: Sender<ServerResponseMsg>,
    },
    _Phantom(PhantomData<LogEntry>),
    _Phantom2(PhantomData<FSM>),
}
