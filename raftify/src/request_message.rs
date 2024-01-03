use raft::eraftpb::{ConfChangeV2, Message as RaftMessage};
use tokio::sync::oneshot::Sender;

use super::{
    response_message::{LocalResponseMsg, ServerResponseMsg},
    AbstractLogEntry, AbstractStateMachine, ClusterJoinTicket,
};

// Request type processed through network calls (grpc)
pub enum ServerRequestMsg {
    MemberBootstrapReady {
        node_id: u64,
        chan: Sender<ServerResponseMsg>,
    },
    ClusterBootstrapReady {
        chan: Sender<ServerResponseMsg>,
    },
    RequestId {
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
}

// Request type used for communication (method calls) between RaftFacade and RaftNode
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
        ticket: ClusterJoinTicket,
        chan: Sender<LocalResponseMsg<LogEntry, FSM>>,
    },
}

// Request type sent from a RaftNode to itself (RaftNode).
// Used for accessing the RaftNode from a future created by RaftNode asynchronous methods
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
    DebugNode,
    Store,
    Storage,
    GetClusterSize,
    Quit,
    Leave,
    MakeSnapshot,
    Propose,
    ChangeConfig,
    SendMessage,
    JoinCluster
);
