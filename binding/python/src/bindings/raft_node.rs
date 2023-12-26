use super::{
    peers::PyPeers,
    state_machine::{PyFSM, PyLogEntry},
};
use pyo3::prelude::*;
use raftify::RaftNode;

#[derive(Clone)]
#[pyclass(name = "RaftNode")]
pub struct PyRaftNode {
    pub inner: RaftNode<PyLogEntry, PyFSM>,
}

#[pymethods]
impl PyRaftNode {
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
}
