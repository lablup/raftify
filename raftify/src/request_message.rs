use std::{collections::HashMap, net::SocketAddr};

use tokio::sync::oneshot::Sender;

use super::{
    response_message::{LocalResponseMsg, ServerResponseMsg},
    AbstractLogEntry, AbstractStateMachine, ClusterJoinTicket,
};
use crate::{
    raft::eraftpb::{ConfChangeV2, Message as RaftMessage},
    InitialRole, Peers,
};

/// Request type processed through network calls (gRPC)
#[derive(Debug)]
pub enum ServerRequestMsg {
    RequestId {
        raft_addr: String,
        tx_msg: Sender<ServerResponseMsg>,
    },
    Propose {
        proposal: Vec<u8>,
        tx_msg: Sender<ServerResponseMsg>,
    },
    ChangeConfig {
        conf_change: ConfChangeV2,
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
}

/// Request type used for communication (method calls) between user side and RaftNode
#[derive(Debug)]
pub enum LocalRequestMsg<LogEntry: AbstractLogEntry, FSM: AbstractStateMachine> {
    IsLeader {
        tx_msg: Sender<LocalResponseMsg<LogEntry, FSM>>,
    },
    GetId {
        tx_msg: Sender<LocalResponseMsg<LogEntry, FSM>>,
    },
    GetLeaderId {
        tx_msg: Sender<LocalResponseMsg<LogEntry, FSM>>,
    },
    GetPeers {
        tx_msg: Sender<LocalResponseMsg<LogEntry, FSM>>,
    },
    AddPeer {
        id: u64,
        addr: String,
        tx_msg: Sender<LocalResponseMsg<LogEntry, FSM>>,
        role: Option<InitialRole>,
    },
    AddPeers {
        peers: HashMap<u64, SocketAddr>,
        tx_msg: Sender<LocalResponseMsg<LogEntry, FSM>>,
    },
    DebugNode {
        tx_msg: Sender<LocalResponseMsg<LogEntry, FSM>>,
    },
    Store {
        tx_msg: Sender<LocalResponseMsg<LogEntry, FSM>>,
    },
    Storage {
        tx_msg: Sender<LocalResponseMsg<LogEntry, FSM>>,
    },
    GetClusterSize {
        tx_msg: Sender<LocalResponseMsg<LogEntry, FSM>>,
    },
    Quit {
        tx_msg: Sender<LocalResponseMsg<LogEntry, FSM>>,
    },
    Campaign {
        tx_msg: Sender<LocalResponseMsg<LogEntry, FSM>>,
    },
    Demote {
        term: u64,
        leader_id: u64,
        tx_msg: Sender<LocalResponseMsg<LogEntry, FSM>>,
    },
    TransferLeader {
        node_id: u64,
        tx_msg: Sender<LocalResponseMsg<LogEntry, FSM>>,
    },
    Leave {
        tx_msg: Sender<LocalResponseMsg<LogEntry, FSM>>,
    },
    MakeSnapshot {
        index: u64,
        term: u64,
        tx_msg: Sender<LocalResponseMsg<LogEntry, FSM>>,
    },
    Propose {
        proposal: Vec<u8>,
        tx_msg: Sender<LocalResponseMsg<LogEntry, FSM>>,
    },
    ChangeConfig {
        conf_change: ConfChangeV2,
        tx_msg: Sender<LocalResponseMsg<LogEntry, FSM>>,
    },
    SendMessage {
        message: Box<RaftMessage>,
        tx_msg: Sender<LocalResponseMsg<LogEntry, FSM>>,
    },
    JoinCluster {
        tickets: Vec<ClusterJoinTicket>,
        tx_msg: Sender<LocalResponseMsg<LogEntry, FSM>>,
    },
    LeaveJoint {},
}

/// Request type sent from a RaftNode to itself (RaftNode).
/// Used for accessing the RaftNode from a future created by RaftNode asynchronous methods
#[derive(Debug)]
pub enum SelfMessage {
    ReportUnreachable { node_id: u64 },
}
