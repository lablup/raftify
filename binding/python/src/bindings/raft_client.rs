
use super::fsm::{PyFSM, PyLogEntry};
use pyo3::{prelude::*, types::PyString};
use raftify::{RaftNode, RaftServiceClient, Channel, create_client};

#[derive(Clone)]
#[pyclass(name = "RaftClient")]
pub struct PyRaftClient {
    inner: RaftServiceClient<Channel>,
}

#[pymethods]
impl PyRaftClient {
    // #[new]
    // pub async fn new(addr: &PyString) -> Self {
    //     Self::_new(addr)
    // }
}

impl PyRaftClient {
    pub async fn _new(addr: &PyString) -> Self {
        let addr = addr.to_str().unwrap().to_owned();
        let inner = create_client(&addr).await.unwrap();

        Self { inner }
    }
}

