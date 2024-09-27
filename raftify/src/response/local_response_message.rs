use std::{fmt, marker::PhantomData, sync::Arc};

use crate::{raft::RawNode, StableStorage};
use tokio::sync::Mutex;

use crate::{AbstractLogEntry, AbstractStateMachine, Peers};

use super::{
    server_response_message::{ConfChangeResponseResult, ResponseResult},
    ResponseMessage,
};

pub enum LocalResponseMsg<
    LogEntry: AbstractLogEntry,
    LogStorage: StableStorage + 'static,
    FSM: AbstractStateMachine,
> {
    IsLeader {
        is_leader: bool,
    },
    GetId {
        id: u64,
    },
    GetLeaderId {
        leader_id: u64,
    },
    GetPeers {
        peers: Peers,
    },
    AddPeer {},
    AddPeers {},
    GetStateMachine {
        store: FSM,
    },
    GetStorage {
        storage: LogStorage,
    },
    GetClusterSize {
        size: usize,
    },
    GetRawNode {
        raw_node: Arc<Mutex<&'static RawNode<LogStorage>>>,
    },
    Quit {},
    Campaign {},
    MakeSnapshot {},
    JoinCluster {},
    SendMessage {},
    Demote {},
    // LeaveJoint {},
    TransferLeader {},
    DebugNode {
        result_json: String,
    },
    _Phantom(PhantomData<LogEntry>),

    // Rerouting available
    Propose {
        result: ResponseResult,
    },
    ConfigChange {
        result: ConfChangeResponseResult,
    },
}

impl<LogEntry: AbstractLogEntry, LogStorage: StableStorage, FSM: AbstractStateMachine> fmt::Debug
    for LocalResponseMsg<LogEntry, LogStorage, FSM>
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

impl<LogEntry: AbstractLogEntry, LogStorage: StableStorage, FSM: AbstractStateMachine>
    From<LocalResponseMsg<LogEntry, LogStorage, FSM>>
    for ResponseMessage<LogEntry, LogStorage, FSM>
{
    fn from(msg: LocalResponseMsg<LogEntry, LogStorage, FSM>) -> Self {
        ResponseMessage::Local(msg)
    }
}
