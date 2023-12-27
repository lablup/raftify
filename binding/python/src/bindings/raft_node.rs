use super::{
    peers::PyPeers,
    raft_rs::eraftpb::conf_change_v2::PyConfChangeV2,
    state_machine::{PyFSM, PyLogEntry},
};
use pyo3::{prelude::*, types::PyString};
use raftify::raft::eraftpb::ConfChangeV2;
use raftify::RaftNode;

#[derive(Clone)]
enum Arguments {
    Propose { proposal: Vec<u8> },
    AddPeer { id: u64, addr: String },
    Empty,
    ChangeConfig { conf_change: ConfChangeV2 },
}

impl Default for Arguments {
    fn default() -> Self {
        Arguments::Empty
    }
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
            args: Arguments::default(),
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
            _ => panic!("Invalid arguments"),
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
            Arguments::Propose { ref proposal } => {
                self.inner.propose(proposal.clone()).await;
            }
            _ => panic!("Invalid arguments"),
        }
        Ok(())
    }

    pub fn prepare_change_config(&mut self, conf_change: &PyConfChangeV2) {
        self.args = Arguments::ChangeConfig {
            conf_change: conf_change.inner.clone(),
        };
    }

    pub async fn change_config(&mut self) {
        match self.args {
            Arguments::ChangeConfig { ref conf_change } => {
                self.inner.change_config(conf_change.clone()).await;
            }
            _ => panic!("Invalid arguments"),
        }
    }

    pub async fn leave(&mut self) {
        self.inner.leave().await;
    }

    pub async fn quit(&mut self) {
        self.inner.quit().await;
    }
}
