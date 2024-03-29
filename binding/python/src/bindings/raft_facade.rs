use pyo3::{exceptions::PyException, prelude::*, types::PyString};
use pyo3_asyncio::tokio::future_into_py;
use raftify::Raft;
use std::sync::Arc;

use super::{
    cluster_join_ticket::PyClusterJoinTicket,
    config::PyConfig,
    logger::PyLogger,
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
    pub fn bootstrap(
        node_id: u64,
        addr: &PyString,
        fsm: PyObject,
        config: PyConfig,
        logger: PyObject,
    ) -> PyResult<Self> {
        let fsm = PyFSM::new(fsm);
        let addr = addr.to_string();

        let raft = Raft::bootstrap(
            node_id,
            addr,
            fsm,
            config.into(),
            Arc::new(PyLogger::new(logger)),
        )
        .unwrap();

        Ok(Self { inner: raft })
    }

    #[staticmethod]
    pub fn request_id<'a>(
        raft_addr: String,
        peer_addr: String,
        py: Python<'a>,
    ) -> PyResult<&'a PyAny> {
        future_into_py(py, async move {
            let ticket = Raft::<PyLogEntry, PyFSM>::request_id(raft_addr, peer_addr.to_owned())
                .await
                .unwrap();
            Ok(PyClusterJoinTicket { inner: ticket })
        })
    }

    pub fn get_raft_node(&self) -> PyRaftNode {
        PyRaftNode::new(self.inner.clone().raft_node.clone())
    }

    pub fn join_cluster<'a>(
        &'a self,
        tickets: Vec<PyClusterJoinTicket>,
        py: Python<'a>,
    ) -> PyResult<&'a PyAny> {
        let raft_facade = self.clone();
        let tickets = tickets.into_iter().map(|t| t.inner).collect();

        future_into_py(py, async move {
            raft_facade.inner.join_cluster(tickets).await;
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
