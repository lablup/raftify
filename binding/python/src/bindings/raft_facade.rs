use std::sync::Arc;

use pyo3::{prelude::*, types::PyString};
use raftify::raft::default_logger;
use raftify::{ClusterJoinTicket, Error, Raft};
use tokio::task::JoinHandle;

use super::config::PyConfig;
use super::peers::PyPeers;
use super::raft_node::PyRaftNode;
use super::state_machine::{PyFSM, PyLogEntry};

use lazy_static::lazy_static;
use tokio::runtime::Runtime;

lazy_static! {
    static ref TOKIO_RT: Runtime = Runtime::new().unwrap();
}

#[derive(Clone)]
enum Arguments {
    Empty,
}

#[derive(Clone)]
#[pyclass(name = "Raft")]
pub struct PyRaftFacade {
    raft: Raft<PyLogEntry, PyFSM>,
    join_ticket: Option<ClusterJoinTicket>,
    raft_task: Option<Arc<JoinHandle<Result<(), Error>>>>,
    args: Arguments,
}

#[derive(Clone)]
#[pyclass(name = "ClusterJoinTicket")]
pub struct PyClusterJoinTicket {
    inner: ClusterJoinTicket,
}

#[pymethods]
impl PyClusterJoinTicket {
    pub fn __repr__(&self) -> PyResult<String> {
        Ok(format!("{:?}", self.inner))
    }

    pub fn get_reserved_id(&self) -> u64 {
        self.inner.reserved_id
    }
}

#[pymethods]
impl PyRaftFacade {
    #[staticmethod]
    pub fn build(
        addr: &PyString,
        fsm: PyObject,
        config: PyConfig,
        join_ticket: Option<PyClusterJoinTicket>,
        initial_peers: Option<PyPeers>,
        // logger: PyObject,
    ) -> PyResult<Self> {
        let fsm = PyFSM::new(fsm);
        let logger = default_logger();
        let addr = addr.to_string();
        let initial_peers = initial_peers.map(|peers| peers.inner);

        let node_id = match join_ticket {
            Some(ref ticket) => ticket.inner.reserved_id,
            None => 1,
        };

        let raft = Raft::build(node_id, addr, fsm, config.into(), logger, initial_peers).unwrap();

        Ok(Self {
            raft,
            join_ticket: join_ticket.map(|t| t.inner),
            raft_task: None,
            args: Arguments::Empty,
        })
    }

    pub async fn run(&mut self) -> PyResult<()> {
        self._run().await
    }

    #[staticmethod]
    pub async fn request_id(peer_addr: String) -> PyClusterJoinTicket {
        Self::_request_id(&peer_addr.to_string()).await
    }

    pub fn is_finished(&self) -> bool {
        match self.raft_task {
            None => false,
            Some(ref task) => task.is_finished(),
        }
    }

    pub fn get_raft_node(&self) -> PyRaftNode {
        PyRaftNode::new(self.raft.clone().raft_node.clone())
    }
}

impl PyRaftFacade {
    async fn _run(&mut self) -> PyResult<()> {
        let raft = self.raft.clone();

        let raft_task = TOKIO_RT.spawn(raft.clone().run());

        if !self.join_ticket.is_none() {
            raft.join(self.join_ticket.clone().unwrap()).await;
        }

        self.raft_task = Some(Arc::new(raft_task));
        Ok(())
    }

    async fn _request_id(peer_addr: &str) -> PyClusterJoinTicket {
        let ticket = TOKIO_RT
            .spawn(Raft::<PyLogEntry, PyFSM>::request_id(peer_addr.to_owned()))
            .await
            .unwrap()
            .unwrap();
        PyClusterJoinTicket { inner: ticket }
    }
}
