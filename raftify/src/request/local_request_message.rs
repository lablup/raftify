use std::{collections::HashMap, net::SocketAddr};

use jopemachine_raft::eraftpb::{ConfChangeV2, Message as RaftMessage};
use tokio::sync::oneshot::Sender;

use crate::{
    response::local_response_message::LocalResponseMsg, AbstractLogEntry, AbstractStateMachine,
    ClusterJoinTicket, InitialRole,
};

use super::common::confchange_request::ConfChangeRequest;

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
    GetStateMachine {
        tx_msg: Sender<LocalResponseMsg<LogEntry, FSM>>,
    },
    GetStorage {
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
        conf_change: ConfChangeRequest,
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
