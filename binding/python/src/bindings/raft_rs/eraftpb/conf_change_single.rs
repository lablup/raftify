use pyo3::{prelude::*, types::PyDict};
use raftify::raft::eraftpb::ConfChangeSingle;

use super::conf_change_type::PyConfChangeType;

#[derive(Clone)]
#[pyclass(name = "ConfChangeSingle")]
pub struct PyConfChangeSingle {
    pub inner: ConfChangeSingle,
}

#[pymethods]
impl PyConfChangeSingle {
    #[new]
    pub fn new() -> Self {
        PyConfChangeSingle {
            inner: ConfChangeSingle::default(),
        }
    }

    pub fn __repr__(&self) -> String {
        format!("{:?}", self.inner)
    }
}

#[pymethods]
impl PyConfChangeSingle {
    pub fn to_dict(&mut self, py: Python) -> PyResult<PyObject> {
        let node_id = self.get_node_id();
        let change_type = self.get_change_type().__repr__();

        let res = PyDict::new(py);
        res.set_item("node_id", node_id).unwrap();
        res.set_item("change_type", change_type).unwrap();
        Ok(res.into_py(py))
    }

    pub fn get_node_id(&self) -> u64 {
        self.inner.get_node_id()
    }

    pub fn set_node_id(&mut self, v: u64) {
        self.inner.set_node_id(v)
    }

    pub fn get_change_type(&self) -> PyConfChangeType {
        PyConfChangeType(self.inner.get_change_type())
    }

    pub fn set_change_type(&mut self, v: &PyConfChangeType) {
        self.inner.set_change_type(v.0)
    }
}
