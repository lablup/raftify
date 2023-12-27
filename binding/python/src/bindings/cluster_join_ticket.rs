use pyo3::prelude::*;
use raftify::ClusterJoinTicket;

#[derive(Clone)]
#[pyclass(name = "ClusterJoinTicket")]
pub struct PyClusterJoinTicket {
    pub inner: ClusterJoinTicket,
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
