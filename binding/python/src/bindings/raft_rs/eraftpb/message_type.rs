use pyo3::{exceptions::PyRuntimeError, prelude::*, pyclass::CompareOp};
use raftify::raft::eraftpb::MessageType;

#[derive(Clone)]
#[pyclass(frozen, name = "MessageType")]
pub struct PyMessageType(pub MessageType);

impl From<PyMessageType> for MessageType {
    fn from(val: PyMessageType) -> Self {
        val.0
    }
}

impl From<MessageType> for PyMessageType {
    fn from(x: MessageType) -> Self {
        match x {
            MessageType::MsgHup => PyMessageType(MessageType::MsgHup),
            MessageType::MsgBeat => PyMessageType(MessageType::MsgBeat),
            MessageType::MsgPropose => PyMessageType(MessageType::MsgPropose),
            MessageType::MsgAppend => PyMessageType(MessageType::MsgAppend),
            MessageType::MsgAppendResponse => PyMessageType(MessageType::MsgAppendResponse),
            MessageType::MsgRequestVote => PyMessageType(MessageType::MsgRequestVote),
            MessageType::MsgRequestVoteResponse => {
                PyMessageType(MessageType::MsgRequestVoteResponse)
            }
            MessageType::MsgSnapshot => PyMessageType(MessageType::MsgSnapshot),
            MessageType::MsgHeartbeat => PyMessageType(MessageType::MsgHeartbeat),
            MessageType::MsgHeartbeatResponse => PyMessageType(MessageType::MsgHeartbeatResponse),
            MessageType::MsgUnreachable => PyMessageType(MessageType::MsgUnreachable),
            MessageType::MsgSnapStatus => PyMessageType(MessageType::MsgSnapStatus),
            MessageType::MsgCheckQuorum => PyMessageType(MessageType::MsgCheckQuorum),
            MessageType::MsgTransferLeader => PyMessageType(MessageType::MsgTransferLeader),
            MessageType::MsgTimeoutNow => PyMessageType(MessageType::MsgTimeoutNow),
            MessageType::MsgReadIndex => PyMessageType(MessageType::MsgReadIndex),
            MessageType::MsgReadIndexResp => PyMessageType(MessageType::MsgReadIndexResp),
            MessageType::MsgRequestPreVote => PyMessageType(MessageType::MsgRequestPreVote),
            MessageType::MsgRequestPreVoteResponse => {
                PyMessageType(MessageType::MsgRequestPreVoteResponse)
            }
        }
    }
}

#[pymethods]
impl PyMessageType {
    pub fn __richcmp__(&self, py: Python, rhs: &PyMessageType, op: CompareOp) -> PyObject {
        match op {
            CompareOp::Eq => (self.0 == rhs.0).into_py(py),
            CompareOp::Ne => (self.0 != rhs.0).into_py(py),
            _ => unimplemented!(),
        }
    }

    pub fn __hash__(&self) -> u64 {
        self.0 as u64
    }

    pub fn __repr__(&self) -> String {
        match self.0 {
            MessageType::MsgHup => "MsgHup".to_string(),
            MessageType::MsgBeat => "MsgBeat".to_string(),
            MessageType::MsgPropose => "MsgPropose".to_string(),
            MessageType::MsgAppend => "MsgAppend".to_string(),
            MessageType::MsgAppendResponse => "MsgAppendResponse".to_string(),
            MessageType::MsgRequestVote => "MsgRequestVote".to_string(),
            MessageType::MsgRequestVoteResponse => "MsgRequestVoteResponse".to_string(),
            MessageType::MsgSnapshot => "MsgSnapshot".to_string(),
            MessageType::MsgHeartbeat => "MsgHeartbeat".to_string(),
            MessageType::MsgHeartbeatResponse => "MsgHeartbeatResponse".to_string(),
            MessageType::MsgUnreachable => "MsgUnreachable".to_string(),
            MessageType::MsgSnapStatus => "MsgSnapStatus".to_string(),
            MessageType::MsgCheckQuorum => "MsgCheckQuorum".to_string(),
            MessageType::MsgTransferLeader => "MsgTransferLeader".to_string(),
            MessageType::MsgTimeoutNow => "MsgTimeoutNow".to_string(),
            MessageType::MsgReadIndex => "MsgReadIndex".to_string(),
            MessageType::MsgReadIndexResp => "MsgReadIndexResp".to_string(),
            MessageType::MsgRequestPreVote => "MsgRequestPreVote".to_string(),
            MessageType::MsgRequestPreVoteResponse => "MsgRequestPreVoteResponse".to_string(),
        }
    }

    pub fn __int__(&self) -> u64 {
        self.0 as u64
    }

    #[staticmethod]
    pub fn from_int(v: i32, py: Python) -> PyResult<PyObject> {
        MessageType::from_i32(v)
            .map(|x| PyMessageType(x).into_py(py))
            .ok_or_else(|| PyRuntimeError::new_err("Invalid value"))
    }

    #[classattr]
    pub fn MsgHup() -> Self {
        PyMessageType(MessageType::MsgHup)
    }

    #[classattr]
    pub fn MsgBeat() -> Self {
        PyMessageType(MessageType::MsgBeat)
    }

    #[classattr]
    pub fn MsgPropose() -> Self {
        PyMessageType(MessageType::MsgPropose)
    }

    #[classattr]
    pub fn MsgAppend() -> Self {
        PyMessageType(MessageType::MsgAppend)
    }

    #[classattr]
    pub fn MsgAppendResponse() -> Self {
        PyMessageType(MessageType::MsgAppendResponse)
    }

    #[classattr]
    pub fn MsgRequestVote() -> Self {
        PyMessageType(MessageType::MsgRequestVote)
    }

    #[classattr]
    pub fn MsgRequestVoteResponse() -> Self {
        PyMessageType(MessageType::MsgRequestVoteResponse)
    }

    #[classattr]
    pub fn MsgSnapshot() -> Self {
        PyMessageType(MessageType::MsgSnapshot)
    }

    #[classattr]
    pub fn MsgHeartbeat() -> Self {
        PyMessageType(MessageType::MsgHeartbeat)
    }

    #[classattr]
    pub fn MsgHeartbeatResponse() -> Self {
        PyMessageType(MessageType::MsgHeartbeatResponse)
    }

    #[classattr]
    pub fn MsgUnreachable() -> Self {
        PyMessageType(MessageType::MsgUnreachable)
    }

    #[classattr]
    pub fn MsgSnapStatus() -> Self {
        PyMessageType(MessageType::MsgSnapStatus)
    }

    #[classattr]
    pub fn MsgCheckQuorum() -> Self {
        PyMessageType(MessageType::MsgCheckQuorum)
    }

    #[classattr]
    pub fn MsgTransferLeader() -> Self {
        PyMessageType(MessageType::MsgTransferLeader)
    }

    #[classattr]
    pub fn MsgTimeoutNow() -> Self {
        PyMessageType(MessageType::MsgTimeoutNow)
    }

    #[classattr]
    pub fn MsgReadIndex() -> Self {
        PyMessageType(MessageType::MsgReadIndex)
    }

    #[classattr]
    pub fn MsgReadIndexResp() -> Self {
        PyMessageType(MessageType::MsgReadIndexResp)
    }

    #[classattr]
    pub fn MsgRequestPreVote() -> Self {
        PyMessageType(MessageType::MsgRequestPreVote)
    }

    #[classattr]
    pub fn MsgRequestPreVoteResponse() -> Self {
        PyMessageType(MessageType::MsgRequestPreVoteResponse)
    }
}
