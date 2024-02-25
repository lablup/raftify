use std::{fmt, marker::PhantomData};

use super::{AbstractLogEntry, AbstractStateMachine, Error, HeedStorage, Peers};

pub enum ResponseMessage<LogEntry: AbstractLogEntry, FSM: AbstractStateMachine> {
    Server(ServerResponseMsg),
    Local(LocalResponseMsg<LogEntry, FSM>),
    _Phantom(PhantomData<LogEntry>),
}

impl<LogEntry: AbstractLogEntry, FSM: AbstractStateMachine> From<LocalResponseMsg<LogEntry, FSM>>
    for ResponseMessage<LogEntry, FSM>
{
    fn from(msg: LocalResponseMsg<LogEntry, FSM>) -> Self {
        ResponseMessage::Local(msg)
    }
}

impl<LogEntry: AbstractLogEntry, FSM: AbstractStateMachine> From<ServerResponseMsg>
    for ResponseMessage<LogEntry, FSM>
{
    fn from(msg: ServerResponseMsg) -> Self {
        ResponseMessage::Server(msg)
    }
}

#[derive(Debug)]
pub enum ResponseResult {
    Success,
    Error(Error),
    WrongLeader { leader_id: u64, leader_addr: String },
}

#[derive(Debug)]
pub enum ConfChangeResponseResult {
    JoinSuccess {
        assigned_ids: Vec<u64>,
        peers: Peers,
    },
    RemoveSuccess,
    Error(Error),
    WrongLeader {
        leader_id: u64,
        leader_addr: String,
    },
}

#[derive(Debug)]
pub enum RequestIdResponseResult {
    Success {
        reserved_id: u64,
        leader_id: u64,
        peers: Peers,
    },
    Error(Error),
    WrongLeader {
        leader_id: u64,
        leader_addr: String,
    },
}

#[derive(Debug)]
pub enum ServerResponseMsg {
    ReportUnreachable { result: ResponseResult },
    DebugNode { result_json: String },
    GetPeers { peers: Peers },
    SendMessage { result: ResponseResult },
    CreateSnapshot {},

    // Rerouting available
    Propose { result: ResponseResult },
    ConfigChange { result: ConfChangeResponseResult },
    RequestId { result: RequestIdResponseResult },
}

pub enum LocalResponseMsg<LogEntry: AbstractLogEntry, FSM: AbstractStateMachine> {
    IsLeader { is_leader: bool },
    GetId { id: u64 },
    GetLeaderId { leader_id: u64 },
    GetPeers { peers: Peers },
    AddPeer {},
    AddPeers {},
    Store { store: FSM },
    Storage { storage: HeedStorage },
    GetClusterSize { size: usize },
    Quit {},
    MakeSnapshot {},
    JoinCluster {},
    SendMessage {},
    LeaveJoint {},
    DebugNode { result_json: String },
    _Phantom(PhantomData<LogEntry>),

    // Rerouting available
    Propose { result: ResponseResult },
    ConfigChange { result: ConfChangeResponseResult },
}

impl<LogEntry: AbstractLogEntry, FSM: AbstractStateMachine> fmt::Debug
    for LocalResponseMsg<LogEntry, FSM>
{
    #[allow(clippy::recursive_format_impl)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LocalResponseMsg::Store { store: _store } => {
                write!(f, "LocalResponseMsg::Store")
            }
            _ => {
                write!(f, "{:?}", self)
            }
        }
    }
}
