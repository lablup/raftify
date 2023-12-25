use super::{
    peers::PyPeers,
    state_machine::{PyFSM, PyLogEntry},
};
use pyo3::prelude::*;
use raftify::RaftNode;

#[derive(Clone)]
#[pyclass(name = "RaftNode")]
pub struct PyRaftNode {
    _inner: RaftNode<PyLogEntry, PyFSM>,
}

#[pymethods]
impl PyRaftNode {
    pub async fn get_id(&self) -> PyResult<u64> {
        Ok(self._get_id().await)
    }

    pub async fn get_leader_id(&self) -> PyResult<u64> {
        Ok(self._get_id().await)
    }

    pub async fn get_peers(&self) -> PyResult<PyPeers> {
        Ok(self._get_peers().await)
    }
}

impl PyRaftNode {
    async fn _get_id(&self) -> u64 {
        self._inner.get_id().await
    }

    async fn _get_leader_id(&self) -> u64 {
        self._inner.get_leader_id().await
    }

    async fn _get_peers(&self) -> PyPeers {
        let peers = self._inner.get_peers().await;
        PyPeers { inner: peers }
    }
}
