use pyo3::{
    prelude::*,
    types::{PyDict, PyString},
};
use raftify::{Peers, Raft};
use slog::{o, Drain};

use super::config::PyConfig;
use super::fsm::PyFSM;

#[derive(Clone)]
#[pyclass(name = "Raft")]
pub struct RaftFacade {
    raft: Raft<PyFSM>,
}

#[pymethods]
impl RaftFacade {
    #[staticmethod]
    pub fn build(
        py: Python,
        node_id: u64,
        addr: &PyString,
        fsm: PyObject,
        config: PyObject,
        // logger: PyObject,
        initial_peers: Option<PyObject>,
    ) -> PyResult<Self> {
        let fsm = PyFSM::new(fsm);
        let config = config.extract::<PyConfig>(py)?;

        let decorator = slog_term::TermDecorator::new().build();
        let drain = slog_term::FullFormat::new(decorator).build().fuse();
        let drain = slog_async::Async::new(drain).build().fuse();
        let logger = slog::Logger::root(drain, o!());

        let initial_peers = Peers::new();

        let addr = addr.to_string_lossy().to_string();
        let raft = Raft::build(
            node_id,
            addr,
            fsm,
            config.into(),
            logger,
            Some(initial_peers),
        )
        .unwrap();
        Ok(Self { raft })
    }
}
