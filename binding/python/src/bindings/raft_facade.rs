use lazy_static::lazy_static;
use pyo3::{prelude::*, types::PyString};
use raftify::{ClusterJoinTicket, Raft, Result};
use std::sync::Arc;
use tokio::{runtime::Runtime, task::JoinHandle};

use super::cluster_join_ticket::PyClusterJoinTicket;
use super::config::PyConfig;
use super::errors::WrongArgumentError;
use super::logger::PyLogger;
use super::peers::PyPeers;
use super::raft_node::PyRaftNode;
use super::state_machine::{PyFSM, PyLogEntry};

lazy_static! {
    pub static ref TOKIO_RT: Runtime = Runtime::new().unwrap();
}

#[derive(Clone, Debug)]
enum Arguments {
    Join { ticket: ClusterJoinTicket },
    MemberBootstrapReady { leader_addr: String, node_id: u64 },
    Empty,
}

#[derive(Clone)]
#[pyclass(name = "Raft")]
pub struct PyRaftFacade {
    raft: Raft<PyLogEntry, PyFSM>,
    raft_task: Option<Arc<JoinHandle<Result<()>>>>,
    args: Arguments,
}

#[pymethods]
impl PyRaftFacade {
    #[staticmethod]
    pub fn build(
        node_id: u64,
        addr: &PyString,
        fsm: PyObject,
        config: PyConfig,
        logger: PyLogger,
        initial_peers: Option<PyPeers>,
    ) -> PyResult<Self> {
        let fsm = PyFSM::new(fsm);
        let addr = addr.to_string();
        let initial_peers = initial_peers.map(|peers| peers.inner);

        let raft = Raft::build(
            node_id,
            addr,
            fsm,
            config.into(),
            logger.inner,
            initial_peers,
        )
        .unwrap();

        Ok(Self {
            raft,
            raft_task: None,
            args: Arguments::Empty,
        })
    }

    #[staticmethod]
    pub async fn request_id(peer_addr: String) -> PyClusterJoinTicket {
        Self::_request_id(&peer_addr.to_string()).await
    }

    pub async fn run(&mut self) {
        self._run().await
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

    pub fn prepare_join(&mut self, ticket: PyClusterJoinTicket) {
        self.args = Arguments::Join {
            ticket: ticket.inner,
        };
    }

    pub async fn join(&mut self) -> PyResult<()> {
        match self.args {
            Arguments::Join { ref ticket } => Ok(self.raft.join(ticket.clone()).await),
            _ => Err(WrongArgumentError::new_err(format!(
                "Invalid arguments {:?}",
                self.args
            ))),
        }
    }

    pub fn prepare_member_bootstrap_ready(&mut self, leader_addr: &PyString, node_id: u64) {
        self.args = Arguments::MemberBootstrapReady {
            leader_addr: leader_addr.to_string(),
            node_id,
        };
    }

    pub async fn member_bootstrap_ready(&mut self) -> PyResult<()> {
        match self.args {
            Arguments::MemberBootstrapReady {
                ref leader_addr,
                node_id,
            } => Ok(self._member_bootstrap_ready(leader_addr, node_id).await),
            _ => Err(WrongArgumentError::new_err(format!(
                "Invalid arguments {:?}",
                self.args
            ))),
        }
    }
}

impl PyRaftFacade {
    async fn _run(&mut self) {
        let raft = self.raft.clone();
        let raft_task = TOKIO_RT.spawn(raft.clone().run());
        self.raft_task = Some(Arc::new(raft_task));
    }

    async fn _request_id(peer_addr: &str) -> PyClusterJoinTicket {
        let ticket = TOKIO_RT
            .spawn(Raft::<PyLogEntry, PyFSM>::request_id(peer_addr.to_owned()))
            .await
            .unwrap()
            .unwrap();
        PyClusterJoinTicket { inner: ticket }
    }

    async fn _member_bootstrap_ready(&self, leader_addr: &str, node_id: u64) {
        let leader_addr = leader_addr.to_owned();
        TOKIO_RT.spawn(Raft::<PyLogEntry, PyFSM>::member_bootstrap_ready(
            leader_addr,
            node_id,
        ));
    }
}
