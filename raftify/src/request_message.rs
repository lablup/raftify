use std::{collections::HashMap, net::SocketAddr};

use tokio::sync::oneshot::Sender;

use super::{
    response_message::{LocalResponseMsg, ServerResponseMsg},
    AbstractLogEntry, AbstractStateMachine, ClusterJoinTicket,
};
use crate::{
    raft::eraftpb::{ConfChangeV2, Message as RaftMessage},
    InitialRole,
};

/// Request type processed through network calls (gRPC)
pub enum ServerRequestMsg {
    RequestId {
        raft_addr: String,
        chan: Sender<ServerResponseMsg>,
    },
    Propose {
        proposal: Vec<u8>,
        chan: Sender<ServerResponseMsg>,
    },
    ChangeConfig {
        conf_change: ConfChangeV2,
        chan: Sender<ServerResponseMsg>,
    },
    DebugNode {
        chan: Sender<ServerResponseMsg>,
    },
    SendMessage {
        message: Box<RaftMessage>,
    },
    GetPeers {
        chan: Sender<ServerResponseMsg>,
    },
    CreateSnapshot {
        chan: Sender<ServerResponseMsg>,
    },
}

/// Request type used for communication (method calls) between user side and RaftNode
pub enum LocalRequestMsg<LogEntry: AbstractLogEntry, FSM: AbstractStateMachine> {
    IsLeader {
        chan: Sender<LocalResponseMsg<LogEntry, FSM>>,
    },
    GetId {
        chan: Sender<LocalResponseMsg<LogEntry, FSM>>,
    },
    GetLeaderId {
        chan: Sender<LocalResponseMsg<LogEntry, FSM>>,
    },
    GetPeers {
        chan: Sender<LocalResponseMsg<LogEntry, FSM>>,
    },
    AddPeer {
        id: u64,
        addr: String,
        chan: Sender<LocalResponseMsg<LogEntry, FSM>>,
        role: Option<InitialRole>,
    },
    AddPeers {
        peers: HashMap<u64, SocketAddr>,
        chan: Sender<LocalResponseMsg<LogEntry, FSM>>,
    },
    DebugNode {
        chan: Sender<LocalResponseMsg<LogEntry, FSM>>,
    },
    Store {
        chan: Sender<LocalResponseMsg<LogEntry, FSM>>,
    },
    Storage {
        chan: Sender<LocalResponseMsg<LogEntry, FSM>>,
    },
    GetClusterSize {
        chan: Sender<LocalResponseMsg<LogEntry, FSM>>,
    },
    Quit {
        chan: Sender<LocalResponseMsg<LogEntry, FSM>>,
    },
    Campaign {
        chan: Sender<LocalResponseMsg<LogEntry, FSM>>,
    },
    Demote {
        term: u64,
        leader_id: u64,
        chan: Sender<LocalResponseMsg<LogEntry, FSM>>,
    },
    TransferLeader {
        node_id: u64,
        chan: Sender<LocalResponseMsg<LogEntry, FSM>>,
    },
    Leave {
        chan: Sender<LocalResponseMsg<LogEntry, FSM>>,
    },
    MakeSnapshot {
        index: u64,
        term: u64,
        chan: Sender<LocalResponseMsg<LogEntry, FSM>>,
    },
    Propose {
        proposal: Vec<u8>,
        chan: Sender<LocalResponseMsg<LogEntry, FSM>>,
    },
    ChangeConfig {
        conf_change: ConfChangeV2,
        chan: Sender<LocalResponseMsg<LogEntry, FSM>>,
    },
    SendMessage {
        message: Box<RaftMessage>,
        chan: Sender<LocalResponseMsg<LogEntry, FSM>>,
    },
    JoinCluster {
        tickets: Vec<ClusterJoinTicket>,
        chan: Sender<LocalResponseMsg<LogEntry, FSM>>,
    },
    LeaveJoint {},
}

/// Request type sent from a RaftNode to itself (RaftNode).
/// Used for accessing the RaftNode from a future created by RaftNode asynchronous methods
pub enum SelfMessage {
    ReportUnreachable { node_id: u64 },
}

macro_rules! impl_debug_for_enum {
    ($enum_name:ident, $($variant:ident),*) => {
        impl<LogEntry: AbstractLogEntry, FSM: AbstractStateMachine> std::fmt::Debug for $enum_name<LogEntry, FSM> {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self {
                    $(
                        $enum_name::$variant { .. } => write!(f, stringify!($variant)),
                    )*
                }
            }
        }
    };
}

impl_debug_for_enum!(
    LocalRequestMsg,
    IsLeader,
    GetId,
    GetLeaderId,
    GetPeers,
    AddPeer,
    AddPeers,
    DebugNode,
    Store,
    Storage,
    GetClusterSize,
    Quit,
    Campaign,
    Demote,
    TransferLeader,
    Leave,
    MakeSnapshot,
    Propose,
    ChangeConfig,
    SendMessage,
    JoinCluster,
    LeaveJoint
);
