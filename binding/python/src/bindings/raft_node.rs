use pyo3::{prelude::*, types::PyString};
use raftify::{
    raft::eraftpb::{ConfChangeV2, Message as RaftMessage},
    RaftNode,
};

use super::{
    errors::WrongArgumentError,
    peers::PyPeers,
    raft_rs::eraftpb::{conf_change_v2::PyConfChangeV2, message::PyMessage},
    state_machine::{PyFSM, PyLogEntry},
};

#[derive(Clone, Debug)]
enum Arguments {
    Propose { proposal: Vec<u8> },
    AddPeer { id: u64, addr: String },
    SendMessage { message: RaftMessage },
    ChangeConfig { conf_change: ConfChangeV2 },
    Empty,
}

#[derive(Clone)]
#[pyclass(name = "RaftNode")]
pub struct PyRaftNode {
    pub inner: RaftNode<PyLogEntry, PyFSM>,
    args: Arguments,
}

impl PyRaftNode {
    pub fn new(inner: RaftNode<PyLogEntry, PyFSM>) -> Self {
        PyRaftNode {
            inner,
            args: Arguments::Empty,
        }
    }
}

#[pymethods]
impl PyRaftNode {
    pub async fn is_leader(&self) -> PyResult<bool> {
        Ok(self.inner.is_leader().await)
    }

    pub async fn get_id(&self) -> PyResult<u64> {
        Ok(self.inner.get_id().await)
    }

    pub async fn get_leader_id(&self) -> PyResult<u64> {
        Ok(self.inner.get_leader_id().await)
    }

    pub async fn get_peers(&self) -> PyResult<PyPeers> {
        let peers = self.inner.get_peers().await;
        Ok(PyPeers { inner: peers })
    }

    pub fn prepare_add_peer(&mut self, id: u64, addr: &PyString) {
        self.args = Arguments::AddPeer {
            id,
            addr: addr.to_string(),
        };
    }

    pub async fn add_peer(&mut self) -> PyResult<()> {
        match self.args {
            Arguments::AddPeer { id, ref addr } => Ok(self.inner.add_peer(id, addr.clone()).await),
            _ => Err(WrongArgumentError::new_err(format!(
                "Invalid arguments {:?}",
                self.args
            ))),
        }
    }

    pub async fn inspect(&self) -> PyResult<String> {
        Ok(format!("{:?}", self.inner.inspect().await))
    }

    pub fn prepare_proposal(&mut self, proposal: Vec<u8>) {
        self.args = Arguments::Propose { proposal };
    }

    pub async fn propose(&mut self) -> PyResult<()> {
        match self.args {
            Arguments::Propose { ref proposal } => Ok(self.inner.propose(proposal.clone()).await),
            _ => Err(WrongArgumentError::new_err(format!(
                "Invalid arguments {:?}",
                self.args
            ))),
        }
    }

    pub fn prepare_change_config(&mut self, conf_change: &PyConfChangeV2) {
        self.args = Arguments::ChangeConfig {
            conf_change: conf_change.inner.clone(),
        };
    }

    pub async fn change_config(&mut self) -> PyResult<()> {
        match self.args {
            Arguments::ChangeConfig { ref conf_change } => {
                // TODO: Define return type and return it.
                self.inner.change_config(conf_change.clone()).await;
                Ok(())
            }
            _ => Err(WrongArgumentError::new_err(format!(
                "Invalid arguments {:?}",
                self.args
            ))),
        }
    }

    pub fn prepare_send_message(&mut self, message: &PyMessage) {
        self.args = Arguments::SendMessage {
            message: message.inner.clone(),
        };
    }

    pub async fn send_message(&mut self) -> PyResult<()> {
        match self.args {
            Arguments::SendMessage { ref message } => {
                self.inner.send_message(message.clone()).await;
                Ok(())
            }
            _ => Err(WrongArgumentError::new_err(format!(
                "Invalid arguments {:?}",
                self.args
            ))),
        }
    }

    pub async fn leave(&mut self) {
        self.inner.leave().await;
    }

    pub async fn quit(&mut self) {
        self.inner.quit().await;
    }

    pub async fn get_cluster_size(&mut self) -> PyResult<usize> {
        Ok(self.inner.get_cluster_size().await)
    }

    pub async fn set_bootstrap_done(&mut self) {
        self.inner.set_bootstrap_done().await
    }

    pub async fn store(&self) -> PyResult<PyFSM> {
        Ok(self.inner.store().await)
    }
}
