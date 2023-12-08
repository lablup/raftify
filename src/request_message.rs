use raft::eraftpb::{ConfChangeV2, Message as RaftMessage};
use tokio::sync::oneshot::Sender;

use crate::{response_message::ResponseMessage, Peers};

pub enum RequestMessage {
    Propose {
        proposal: Vec<u8>,
        chan: Sender<ResponseMessage>,
    },
    ConfigChange {
        conf_change: ConfChangeV2,
        chan: Sender<ResponseMessage>,
    },
    RequestId {
        chan: Sender<ResponseMessage>,
    },
    ReportUnreachable {
        node_id: u64,
    },
    DebugNode {
        chan: Sender<ResponseMessage>,
    },
    RaftMessage {
        message: Box<RaftMessage>,
    },
}
