use pyo3::prelude::*;
use raftify::{create_client, Channel, RaftServiceClient};

#[derive(Clone)]
#[pyclass(name = "RaftClient")]
pub struct PyRaftClient {
    inner: RaftServiceClient<Channel>,
}

#[pymethods]
impl PyRaftClient {
    #[staticmethod]
    pub fn new(addr: String) -> Self {
        Self::_new(&addr)
    }
}

impl PyRaftClient {
    #[tokio::main]
    pub async fn _new(addr: &str) -> Self {
        let addr = addr.to_owned();
        let inner = create_client(&addr).await.unwrap();

        Self { inner }
    }
}
