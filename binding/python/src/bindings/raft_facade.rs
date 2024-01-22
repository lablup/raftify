use pyo3::{exceptions::PyException, prelude::*, types::PyString};
use pyo3_asyncio::tokio::future_into_py;
use raftify::Raft;
use std::sync::Arc;

use super::{
    cluster_join_ticket::PyClusterJoinTicket,
    config::PyConfig,
    logger::PyLogger,
    peers::PyPeers,
    raft_node::PyRaftNode,
    state_machine::{PyFSM, PyLogEntry},
};

#[derive(Clone)]
#[pyclass(name = "Raft")]
pub struct PyRaftFacade {
    inner: Raft<PyLogEntry, PyFSM>,
}

#[pymethods]
impl PyRaftFacade {
    #[staticmethod]
    pub fn bootstrap_cluster(
        node_id: u64,
        addr: &PyString,
        fsm: PyObject,
        config: PyConfig,
        logger: PyObject,
        initial_peers: Option<PyPeers>,
    ) -> PyResult<Self> {
        let fsm = PyFSM::new(fsm);
        let addr = addr.to_string();
        let initial_peers = initial_peers.map(|peers| peers.inner);

        let raft = Raft::bootstrap_cluster(
            node_id,
            addr,
            fsm,
            config.into(),
            initial_peers,
            Arc::new(PyLogger::new(logger)),
        )
        .unwrap();

        Ok(Self { inner: raft })
    }

    #[staticmethod]
    pub fn new_follower(
        node_id: u64,
        addr: &PyString,
        fsm: PyObject,
        config: PyConfig,
        logger: PyObject,
        initial_peers: Option<PyPeers>,
    ) -> PyResult<Self> {
        let fsm = PyFSM::new(fsm);
        let addr = addr.to_string();
        let initial_peers = initial_peers.map(|peers| peers.inner);

        let raft = Raft::new_follower(
            node_id,
            addr,
            fsm,
            config.into(),
            initial_peers,
            Arc::new(PyLogger::new(logger)),
        )
        .unwrap();

        Ok(Self { inner: raft })
    }

    #[staticmethod]
    pub fn request_id<'a>(
        raft_addr: String,
        peer_addr: String,
        logger: PyObject,
        py: Python<'a>,
    ) -> PyResult<&'a PyAny> {
        future_into_py(py, async move {
            let ticket = Raft::<PyLogEntry, PyFSM>::request_id(
                raft_addr,
                peer_addr.to_owned(),
                Arc::new(PyLogger::new(logger)),
            )
            .await
            .unwrap();
            Ok(PyClusterJoinTicket { inner: ticket })
        })
    }

    #[staticmethod]
    pub fn member_bootstrap_ready<'a>(
        leader_addr: String,
        node_id: u64,
        logger: PyObject,
        py: Python<'a>,
    ) -> PyResult<&'a PyAny> {
        future_into_py(py, async move {
            Raft::<PyLogEntry, PyFSM>::member_bootstrap_ready(
                leader_addr,
                node_id,
                Arc::new(PyLogger::new(logger)),
            )
            .await
            .map(|_| ())
            .map_err(|e| PyException::new_err(e.to_string()))
        })
    }

    pub fn get_raft_node(&self) -> PyRaftNode {
        PyRaftNode::new(self.inner.clone().raft_node.clone())
    }

    pub fn join<'a>(&'a self, ticket: PyClusterJoinTicket, py: Python<'a>) -> PyResult<&'a PyAny> {
        let raft_facade = self.clone();

        future_into_py(py, async move {
            raft_facade.inner.join(ticket.inner).await;
            Ok(())
        })
    }

    pub fn run<'a>(&'a self, py: Python<'a>) -> PyResult<&'a PyAny> {
        let raft_facade = self.clone();

        future_into_py(py, async move {
            raft_facade
                .inner
                .run()
                .await
                .map(|_| ())
                .map_err(|e| PyException::new_err(e.to_string()))
        })
    }
}
