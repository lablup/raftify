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
#[pyclass(name = "Raft")]
pub struct PyRaftFacade {
    inner: Raft<PyLogEntry, PyFSM>,
    join_ticket: Option<ClusterJoinTicket>,
    raft_task: Option<Arc<JoinHandle<Result<(), Error>>>>,
    proposal: Option<Vec<u8>>,
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

    pub fn reserved_id(&self) -> u64 {
        self.inner.reserved_id
    }
}

#[pymethods]
impl PyRaftFacade {
    #[staticmethod]
    pub fn build(
        _py: Python,
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
            inner: raft,
            join_ticket: join_ticket.map(|t| t.inner),
            raft_task: None,
            proposal: None,
        })
    }

    pub async fn run(&mut self) -> PyResult<()> {
        self._run().await
    }

    #[staticmethod]
    pub async fn request_id(peer_addr: String) -> PyClusterJoinTicket {
        Self::_request_id(&peer_addr.to_string()).await
    }

    pub async fn cluster_size(&self) -> PyResult<usize> {
        let size = self.inner.cluster_size().await;
        Ok(size)
    }

    pub fn prepare_proposal(&mut self, proposal: Vec<u8>) {
        self.proposal = Some(proposal);
    }

    pub async fn propose(&mut self) -> PyResult<()> {
        self.inner
            .raft_node
            .propose(self.proposal.take().unwrap())
            .await;
        Ok(())
    }

    pub fn is_finished(&self) -> bool {
        self.raft_task.clone().unwrap().is_finished()
    }

    pub fn get_raft_node(&self) -> PyRaftNode {
        let raft_node = self.inner.raft_node.clone();
        PyRaftNode { inner: raft_node }
    }
}

impl PyRaftFacade {
    async fn _run(&mut self) -> PyResult<()> {
        let raft = self.inner.clone();

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
