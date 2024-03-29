use pyo3::{prelude::*, types::PyString};
use pyo3_asyncio::tokio::future_into_py;
use raftify::RaftNode;

use super::{
    peers::PyPeers,
    raft_rs::eraftpb::{conf_change_v2::PyConfChangeV2, message::PyMessage},
    role::PyInitialRole,
    state_machine::{PyFSM, PyLogEntry},
};

#[derive(Clone)]
#[pyclass(name = "RaftNode")]
pub struct PyRaftNode {
    pub inner: RaftNode<PyLogEntry, PyFSM>,
}

impl PyRaftNode {
    pub fn new(inner: RaftNode<PyLogEntry, PyFSM>) -> Self {
        PyRaftNode { inner }
    }
}

#[pymethods]
impl PyRaftNode {
    pub fn is_leader<'a>(&'a self, py: Python<'a>) -> PyResult<&'a PyAny> {
        let raft_node = self.inner.clone();
        future_into_py(py, async move { Ok(raft_node.is_leader().await) })
    }

    pub fn get_id<'a>(&'a self, py: Python<'a>) -> PyResult<&'a PyAny> {
        let raft_node = self.inner.clone();
        future_into_py(py, async move { Ok(raft_node.get_id().await) })
    }

    pub fn get_leader_id<'a>(&'a self, py: Python<'a>) -> PyResult<&'a PyAny> {
        let raft_node = self.inner.clone();
        future_into_py(py, async move { Ok(raft_node.get_leader_id().await) })
    }

    pub fn get_peers<'a>(&'a self, py: Python<'a>) -> PyResult<&'a PyAny> {
        let raft_node = self.inner.clone();

        future_into_py(py, async move {
            let peers = raft_node.get_peers().await;
            Ok(PyPeers { inner: peers })
        })
    }

    pub fn add_peer<'a>(
        &'a self,
        id: u64,
        addr: &PyString,
        role: &PyInitialRole,
        py: Python<'a>,
    ) -> PyResult<&'a PyAny> {
        let raft_node = self.inner.clone();
        let addr = addr.to_string();
        let role = role.0.clone();

        future_into_py(py, async move {
            raft_node.add_peer(id, addr, Some(role)).await;
            Ok(())
        })
    }

    pub fn inspect<'a>(&'a self, py: Python<'a>) -> PyResult<&'a PyAny> {
        let raft_node = self.inner.clone();

        future_into_py(
            py,
            async move { Ok(format!("{:?}", raft_node.inspect().await)) },
        )
    }

    pub fn propose<'a>(&'a self, proposal: Vec<u8>, py: Python<'a>) -> PyResult<&'a PyAny> {
        let raft_node = self.inner.clone();

        future_into_py(py, async move {
            raft_node.propose(proposal.clone()).await;
            Ok(())
        })
    }

    pub fn change_config<'a>(
        &'a self,
        conf_change: &PyConfChangeV2,
        py: Python<'a>,
    ) -> PyResult<&'a PyAny> {
        let raft_node = self.inner.clone();
        let conf_change = conf_change.inner.clone();

        future_into_py(py, async move {
            raft_node.change_config(conf_change).await;
            Ok(())
        })
    }

    pub fn send_message<'a>(&'a self, message: &PyMessage, py: Python<'a>) -> PyResult<&'a PyAny> {
        let raft_node = self.inner.clone();
        let message = message.inner.clone();

        future_into_py(py, async move {
            raft_node.send_message(message).await;
            Ok(())
        })
    }

    pub fn leave<'a>(&'a self, py: Python<'a>) -> PyResult<&'a PyAny> {
        let raft_node = self.inner.clone();

        future_into_py(py, async move {
            raft_node.leave().await;
            Ok(())
        })
    }

    pub fn leave_joint<'a>(&'a self, py: Python<'a>) -> PyResult<&'a PyAny> {
        let raft_node = self.inner.clone();

        future_into_py(py, async move {
            raft_node.leave_joint().await;
            Ok(())
        })
    }

    pub fn demote<'a>(&'a self, term: u64, leader_id: u64, py: Python<'a>) -> PyResult<&'a PyAny> {
        let raft_node = self.inner.clone();

        future_into_py(py, async move {
            raft_node.demote(term, leader_id).await;
            Ok(())
        })
    }

    pub fn transfer_leader<'a>(&'a self, leader_id: u64, py: Python<'a>) -> PyResult<&'a PyAny> {
        let raft_node = self.inner.clone();

        future_into_py(py, async move {
            raft_node.transfer_leader(leader_id).await;
            Ok(())
        })
    }

    pub fn quit<'a>(&'a mut self, py: Python<'a>) -> PyResult<&'a PyAny> {
        let raft_node = self.inner.clone();

        future_into_py(py, async move {
            raft_node.quit().await;
            Ok(())
        })
    }

    pub fn get_cluster_size<'a>(&'a mut self, py: Python<'a>) -> PyResult<&'a PyAny> {
        let raft_node = self.inner.clone();
        future_into_py(py, async move { Ok(raft_node.get_cluster_size().await) })
    }

    pub fn state_machine<'a>(&'a mut self, py: Python<'a>) -> PyResult<&'a PyAny> {
        let raft_node = self.inner.clone();
        future_into_py(py, async move { Ok(raft_node.state_machine().await) })
    }
}
