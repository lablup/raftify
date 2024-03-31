use std::{fmt, marker::PhantomData};

use crate::{AbstractLogEntry, AbstractStateMachine, HeedStorage, Peers};

use super::{
    server_response_message::{ConfChangeResponseResult, ResponseResult},
    ResponseMessage,
};

pub enum LocalResponseMsg<LogEntry: AbstractLogEntry, FSM: AbstractStateMachine> {
    IsLeader { is_leader: bool },
    GetId { id: u64 },
    GetLeaderId { leader_id: u64 },
    GetPeers { peers: Peers },
    AddPeer {},
    AddPeers {},
    GetStateMachine { store: FSM },
    GetStorage { storage: HeedStorage },
    GetClusterSize { size: usize },
    Quit {},
    Campaign {},
    MakeSnapshot {},
    JoinCluster {},
    SendMessage {},
    Demote {},
    // LeaveJoint {},
    TransferLeader {},
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
            LocalResponseMsg::GetStateMachine { store: _store } => {
                write!(f, "LocalResponseMsg::Store")
            }
            _ => {
                write!(f, "{:?}", self)
            }
        }
    }
}

impl<LogEntry: AbstractLogEntry, FSM: AbstractStateMachine> From<LocalResponseMsg<LogEntry, FSM>>
    for ResponseMessage<LogEntry, FSM>
{
    fn from(msg: LocalResponseMsg<LogEntry, FSM>) -> Self {
        ResponseMessage::Local(msg)
    }
}
