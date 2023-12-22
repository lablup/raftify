use pyo3::{prelude::*, types::PyString};
use raftify::raft::default_logger;
use raftify::Raft;

use super::config::PyConfig;
use super::fsm::{PyFSM, PyLogEntry};
use super::peers::PyPeers;

#[derive(Clone)]
#[pyclass(name = "Raft")]
pub struct PyRaftFacade {
    inner: Raft<PyLogEntry, PyFSM>,
}

#[pymethods]
impl PyRaftFacade {
    #[staticmethod]
    pub fn build(
        _py: Python,
        node_id: u64,
        addr: &PyString,
        fsm: PyObject,
        config: PyConfig,
        // logger: PyObject,
        initial_peers: Option<PyPeers>,
    ) -> PyResult<Self> {
        let fsm = PyFSM::new(fsm);
        let logger = default_logger();
        let addr = addr.to_string();
        let initial_peers = initial_peers.map(|peers| peers.inner);

        let raft = Raft::build(node_id, addr, fsm, config.into(), logger, initial_peers).unwrap();

        Ok(Self { inner: raft })
    }

    pub async fn run(&mut self) -> PyResult<()> {
        self._run()
    }
}

impl PyRaftFacade {
    #[tokio::main]
    async fn _run(&mut self) -> PyResult<()> {
        let raft = self.inner.clone();
        match raft.run().await {
            Ok(_) => println!("RaftNode exited successfully"),
            Err(e) => println!("RaftNode exited with error: {}", e),
        };
        Ok(())
    }
}
