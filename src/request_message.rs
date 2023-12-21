use raft::eraftpb::{ConfChangeV2, Message as RaftMessage};
use tokio::sync::oneshot::Sender;

use crate::response_message::{LocalResponseMsg, ServerResponseMsg};

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

pub enum LocalRequestMsg {
    IsLeader {
        chan: Sender<LocalResponseMsg>,
    },
    GetId {
        chan: Sender<LocalResponseMsg>,
    },
    GetLeaderId {
        chan: Sender<LocalResponseMsg>,
    },
    GetPeers {
        chan: Sender<LocalResponseMsg>,
    },
    AddPeer {
        id: u64,
        addr: String,
        chan: Sender<LocalResponseMsg>,
    },
    Inspect {
        chan: Sender<LocalResponseMsg>,
    },
    Store {
        chan: Sender<LocalResponseMsg>,
    },
    Storage {
        chan: Sender<LocalResponseMsg>,
    },
    GetClusterSize {
        chan: Sender<LocalResponseMsg>,
    },
    Quit {},
    MakeSnapshot {
        chan: Sender<LocalResponseMsg>,
    },
    Propose {
        proposal: Vec<u8>,
        chan: Sender<LocalResponseMsg>,
    },
}
