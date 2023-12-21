use raft::eraftpb::{ConfChangeV2, Message as RaftMessage};
use tokio::sync::oneshot::Sender;

use crate::{
    response_message::{LocalResponseMsg, ServerResponseMsg},
    AbstractLogEntry, AbstractStateMachine,
};

pub enum ServerRequestMsg {
    MemberBootstrapReady {
        node_id: u64,
        chan: Sender<ServerResponseMsg>,
    },
    ClusterBootstrapReady {
        chan: Sender<ServerResponseMsg>,
    },
    Propose {
        proposal: Vec<u8>,
        chan: Sender<ServerResponseMsg>,
    },
    ConfigChange {
        conf_change: ConfChangeV2,
        chan: Sender<ServerResponseMsg>,
    },
    RequestId {
        chan: Sender<ServerResponseMsg>,
    },
    ReportUnreachable {
        node_id: u64,
    },
    DebugNode {
        chan: Sender<ServerResponseMsg>,
    },
    RaftMessage {
        message: Box<RaftMessage>,
    },
}

pub enum LocalRequestMsg<LogEntry: AbstractLogEntry, FSM: AbstractStateMachine<LogEntry>> {
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
    Inspect {
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
    MakeSnapshot {
        chan: Sender<LocalResponseMsg<LogEntry, FSM>>,
    },
    Propose {
        proposal: Vec<u8>,
        chan: Sender<LocalResponseMsg<LogEntry, FSM>>,
    },
}
