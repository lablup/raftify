use pyo3::prelude::*;
use pyo3_asyncio::tokio::future_into_py;
use raftify::Peer;

use super::role::PyInitialRole;

#[derive(Clone)]
#[pyclass(name = "Peer")]
pub struct PyPeer {
    pub inner: Peer,
}

#[pymethods]
impl PyPeer {
    #[new]
    pub fn new(addr: String, role: &PyInitialRole) -> Self {
        PyPeer {
            inner: Peer::new(addr, role.0.clone()),
        }
    }

    pub fn __repr__(&self) -> String {
        format!("{:?}", self.inner)
    }

    pub fn get_addr(&self) -> String {
        self.inner.addr.to_string()
    }

    pub fn get_role(&self) -> PyInitialRole {
        PyInitialRole(self.inner.role.clone())
    }

    pub fn connect<'a>(&'a self, py: Python<'a>) -> PyResult<&'a PyAny> {
        let mut peer = self.inner.clone();
        future_into_py(py, async move {
            peer.connect().await;
            Ok(())
        })
    }
}
