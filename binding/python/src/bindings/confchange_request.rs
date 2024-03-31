use std::net::SocketAddr;

use pyo3::{prelude::*, types::PyList};
use raftify::ConfChangeRequest;

use super::raft_rs::eraftpb::conf_change_single::PyConfChangeSingle;

#[derive(Clone)]
#[pyclass(name = "ConfChangeRequest")]
pub struct PyConfChangeRequest {
    pub inner: ConfChangeRequest,
}

#[pymethods]
impl PyConfChangeRequest {
    #[new]
    pub fn new(changes: &PyList, addrs: &PyList) -> PyResult<Self> {
        let changes = changes
            .extract::<Vec<PyConfChangeSingle>>()?
            .into_iter()
            .map(|change| change.inner)
            .collect::<Vec<_>>();

        let addrs = addrs
            .extract::<Vec<String>>()?
            .into_iter()
            .map(|addr| addr.parse().unwrap())
            .collect::<Vec<SocketAddr>>();

        Ok(PyConfChangeRequest {
            inner: ConfChangeRequest { changes, addrs },
        })
    }
}
