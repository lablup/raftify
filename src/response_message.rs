use crate::{Error, HeedStorage, Peers};

pub enum ResponseMessage {
    Server(ServerResponseMsg),
    Local(LocalResponseMsg),
}

impl From<LocalResponseMsg> for ResponseMessage {
    fn from(msg: LocalResponseMsg) -> Self {
        ResponseMessage::Local(msg)
    }
}

impl From<ServerResponseMsg> for ResponseMessage {
    fn from(msg: ServerResponseMsg) -> Self {
        ResponseMessage::Server(msg)
    }
}

#[derive(Debug)]
pub enum ServerResponseResult {
    Success,
    Error(Error),
    WrongLeader { leader_id: u64, leader_addr: String },
}

#[derive(Debug)]
pub enum ServerConfChangeResponseResult {
    JoinSuccess { assigned_id: u64, peers: Peers },
    RemoveSuccess,
    Error(Error),
    WrongLeader { leader_id: u64, leader_addr: String },
}

#[derive(Debug)]
pub enum ServerResponseMsg {
    MemberBootstrapReady {
        result: ServerResponseResult,
    },

    ClusterBootstrapReady {
        result: ServerResponseResult,
    },

    Propose {
        result: ServerResponseResult,
    },

    ConfigChange {
        result: ServerConfChangeResponseResult,
    },

    RequestId {
        result: ServerResponseResult,
        reserved_id: Option<u64>,
        leader_id: Option<u64>,
        leader_addr: Option<String>,
        peers: Option<Peers>,
    },

    ReportUnreachable {
        result: ServerResponseResult,
    },

    DebugNode {
        result: String,
    },

    RaftMessage {
        result: ServerResponseResult,
    },
}

#[derive(Debug)]
pub enum LocalResponseMsg {
    IsLeader { is_leader: bool },
    GetId { id: u64 },
    GetLeaderId { leader_id: u64 },
    GetPeers { peers: Peers },
    AddPeer {},
    Inspect { result: String },
    Storage { storage: HeedStorage },
    GetClusterSize { size: usize },
    Quit {},
    MakeSnapshot {},
    Propose {},
}
