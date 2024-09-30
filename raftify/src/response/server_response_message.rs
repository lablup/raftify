use crate::{AbstractLogEntry, AbstractStateMachine, Error, Peers, StableStorage};

use super::ResponseMessage;

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
    SetPeers {},
    SendMessage { result: ResponseResult },
    CreateSnapshot {},
    LeaveJoint {},
    JoinCluster {},

    // Rerouting available
    Propose { result: ResponseResult },
    ConfigChange { result: ConfChangeResponseResult },
    RequestId { result: RequestIdResponseResult },
}

impl<LogEntry: AbstractLogEntry, LogStorage: StableStorage, FSM: AbstractStateMachine>
    From<ServerResponseMsg> for ResponseMessage<LogEntry, LogStorage, FSM>
{
    fn from(msg: ServerResponseMsg) -> Self {
        ResponseMessage::Server(msg)
    }
}
