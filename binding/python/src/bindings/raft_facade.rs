use pyo3::{prelude::*, types::PyString};
use raftify::raft::default_logger;
use raftify::{ClusterJoinTicket, Raft};

use super::config::PyConfig;
use super::fsm::{PyFSM, PyLogEntry};
use super::peers::PyPeers;

#[derive(Clone)]
#[pyclass(name = "Raft")]
pub struct PyRaftFacade {
    inner: Raft<PyLogEntry, PyFSM>,
    join_ticket: Option<ClusterJoinTicket>,
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
        // logger: PyObject,
        join_ticket: Option<PyClusterJoinTicket>,
        initial_peers: Option<PyPeers>,
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
        })
    }

    pub async fn run(&self) -> PyResult<()> {
        self._run()
    }

    // // TODO: Remove this and pass ClusterJoinTicket to `join` directly after it become possible
    // pub fn prepare_join(&mut self, ticket: &PyClusterJoinTicket) -> PyResult<()> {
    //     self.join_ticket = Some(ticket.inner.clone());
    //     Ok(())
    // }

    #[staticmethod]
    pub async fn request_id(peer_addr: String) -> PyClusterJoinTicket {
        Self::_request_id(&peer_addr.to_string())
    }

    // run이 실행된 상태에서의 &mut self가 두 개 이상 존재하게 되므로 데드락
    // 채널을 통해 우회하자 -> 여전히 데드락. 어디에서 데드락이 걸리는 지 확실하지 않음.
    // oneshot::send -> 어째서 여기서 블로킹?
    pub async fn join(&self) -> PyResult<()> {
        assert!(self.join_ticket.is_some());

        let ticket = self.join_ticket.clone().unwrap();
        println!("join 2!!");
        self._join(ticket)
    }

    pub async fn cluster_size(&self) -> PyResult<usize> {
        let size = self.inner.cluster_size().await;
        Ok(size)
    }
}

impl PyRaftFacade {
    #[tokio::main]
    async fn _run(&self) -> PyResult<()> {
        let raft = self.inner.clone();
        raft.run().await.unwrap();
        Ok(())
    }

    #[tokio::main]
    async fn _request_id(peer_addr: &str) -> PyClusterJoinTicket {
        let ticket = Raft::<PyLogEntry, PyFSM>::request_id(peer_addr.to_owned())
            .await
            .unwrap();
        PyClusterJoinTicket { inner: ticket }
    }

    #[tokio::main]
    async fn _join(&self, ticket: ClusterJoinTicket) -> PyResult<()> {
        println!("Joining cluster with ticket: {:?}", ticket);
        self.inner.join(ticket).await;
        println!("Joining cluster with ticket: 2");
        Ok(())
    }
}
