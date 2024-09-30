use std::{collections::HashMap, net::SocketAddr};

use crate::{raft::eraftpb::Message as RaftMessage, StableStorage};
use tokio::sync::oneshot::Sender;

use crate::{
    response::local_response_message::LocalResponseMsg, AbstractLogEntry, AbstractStateMachine,
    ClusterJoinTicket, InitialRole,
};

use super::common::confchange_request::ConfChangeRequest;

/// Request type used for communication (method calls) between user side and RaftNode
#[derive(Debug)]
pub enum LocalRequestMsg<
    LogEntry: AbstractLogEntry,
    LogStorage: StableStorage + 'static,
    FSM: AbstractStateMachine,
> {
    IsLeader {
        tx_msg: Sender<LocalResponseMsg<LogEntry, LogStorage, FSM>>,
    },
    GetId {
        tx_msg: Sender<LocalResponseMsg<LogEntry, LogStorage, FSM>>,
    },
    GetLeaderId {
        tx_msg: Sender<LocalResponseMsg<LogEntry, LogStorage, FSM>>,
    },
    GetPeers {
        tx_msg: Sender<LocalResponseMsg<LogEntry, LogStorage, FSM>>,
    },
    AddPeer {
        id: u64,
        addr: String,
        tx_msg: Sender<LocalResponseMsg<LogEntry, LogStorage, FSM>>,
        role: Option<InitialRole>,
    },
    AddPeers {
        peers: HashMap<u64, SocketAddr>,
        tx_msg: Sender<LocalResponseMsg<LogEntry, LogStorage, FSM>>,
    },
    DebugNode {
        tx_msg: Sender<LocalResponseMsg<LogEntry, LogStorage, FSM>>,
    },
    GetStateMachine {
        tx_msg: Sender<LocalResponseMsg<LogEntry, LogStorage, FSM>>,
    },
    GetStorage {
        tx_msg: Sender<LocalResponseMsg<LogEntry, LogStorage, FSM>>,
    },
    GetClusterSize {
        tx_msg: Sender<LocalResponseMsg<LogEntry, LogStorage, FSM>>,
    },
    Quit {
        tx_msg: Sender<LocalResponseMsg<LogEntry, LogStorage, FSM>>,
    },
    Campaign {
        tx_msg: Sender<LocalResponseMsg<LogEntry, LogStorage, FSM>>,
    },
    Demote {
        term: u64,
        leader_id: u64,
        tx_msg: Sender<LocalResponseMsg<LogEntry, LogStorage, FSM>>,
    },
    TransferLeader {
        node_id: u64,
        tx_msg: Sender<LocalResponseMsg<LogEntry, LogStorage, FSM>>,
    },
    Leave {
        tx_msg: Sender<LocalResponseMsg<LogEntry, LogStorage, FSM>>,
    },
    MakeSnapshot {
        index: u64,
        term: u64,
        tx_msg: Sender<LocalResponseMsg<LogEntry, LogStorage, FSM>>,
    },
    Propose {
        proposal: Vec<u8>,
        tx_msg: Sender<LocalResponseMsg<LogEntry, LogStorage, FSM>>,
    },
    ChangeConfig {
        conf_change: ConfChangeRequest,
        tx_msg: Sender<LocalResponseMsg<LogEntry, LogStorage, FSM>>,
    },
    SendMessage {
        message: Box<RaftMessage>,
        tx_msg: Sender<LocalResponseMsg<LogEntry, LogStorage, FSM>>,
    },
    JoinCluster {
        tickets: Vec<ClusterJoinTicket>,
        tx_msg: Sender<LocalResponseMsg<LogEntry, LogStorage, FSM>>,
    },
    LeaveJoint {},
    GetRawNode {
        tx_msg: Sender<LocalResponseMsg<LogEntry, LogStorage, FSM>>,
    },
}
