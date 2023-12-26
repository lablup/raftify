use pyo3::types::PyDict;
use pyo3::{prelude::*};

use super::conf_change_type::PyConfChangeType;

use raftify::raft::eraftpb::ConfChangeSingle;

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
        let node_id = self.get_node_id()?;
        let change_type = self.get_change_type()?.__repr__();

        let res = PyDict::new(py);
        res.set_item("node_id", node_id).unwrap();
        res.set_item("change_type", change_type).unwrap();
        Ok(res.into_py(py))
    }

    pub fn get_node_id(&self) -> PyResult<u64> {
        self.get_node_id()
    }

    pub fn set_node_id(&mut self, v: u64) -> PyResult<()> {
        self.set_node_id(v)
    }

    pub fn clear_node_id(&mut self) -> PyResult<()> {
        self.clear_node_id()
    }

    pub fn get_change_type(&self) -> PyResult<PyConfChangeType> {
        self.get_change_type()
    }

    pub fn set_change_type(&mut self, v: &PyConfChangeType) -> PyResult<()> {
        self.set_change_type(v)
    }

    pub fn clear_change_type(&mut self) -> PyResult<()> {
        self.clear_change_type()
    }
}
