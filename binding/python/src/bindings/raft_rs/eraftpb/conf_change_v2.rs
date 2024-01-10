use pyo3::{
    prelude::*,
    types::{PyBytes, PyDict, PyList},
};
use raftify::raft::{eraftpb::ConfChangeV2, formatter::format_confchangev2};

use super::{
    conf_change_single::PyConfChangeSingle, conf_change_transition::PyConfChangeTransition,
};

#[derive(Clone)]
#[pyclass(name = "ConfChangeV2")]
pub struct PyConfChangeV2 {
    pub inner: ConfChangeV2,
}

#[pymethods]
impl PyConfChangeV2 {
    #[new]
    pub fn new() -> Self {
        PyConfChangeV2 {
            inner: ConfChangeV2::default(),
        }
    }

    pub fn __repr__(&self) -> String {
        format_confchangev2(&self.inner)
    }
}

impl Default for PyConfChangeV2 {
    fn default() -> Self {
        Self::new()
    }
}

#[pymethods]
impl PyConfChangeV2 {
    pub fn to_dict(&mut self, py: Python) -> PyResult<PyObject> {
        let context = self.get_context(py);
        let transition: String = self.get_transition().__repr__();

        let changes = self
            .inner
            .get_changes()
            .iter()
            .map(|cs| {
                PyConfChangeSingle { inner: cs.clone() }
                    .to_dict(py)
                    .unwrap()
            })
            .collect::<Vec<_>>();
        let changes = PyList::new(py, changes);

        let res = PyDict::new(py);
        res.set_item("changes", changes).unwrap();
        res.set_item("context", context).unwrap();
        res.set_item("transition", transition).unwrap();
        Ok(res.into_py(py))
    }

    pub fn get_changes(&self, py: Python) -> PyObject {
        self.inner
            .get_changes()
            .iter()
            .map(|cs| PyConfChangeSingle { inner: cs.clone() })
            .collect::<Vec<_>>()
            .into_py(py)
    }

    pub fn set_changes(&mut self, v: &PyList) {
        self.inner.set_changes(
            v.iter()
                .map(|cs| cs.extract::<PyConfChangeSingle>().unwrap().inner)
                .collect::<Vec<_>>(),
        )
    }

    pub fn get_context(&self, py: Python) -> Py<PyBytes> {
        PyBytes::new(py, self.inner.get_context()).into_py(py)
    }

    pub fn set_context(&mut self, v: &PyAny) {
        self.inner.set_context(v.extract::<Vec<u8>>().unwrap());
    }

    pub fn get_transition(&self) -> PyConfChangeTransition {
        PyConfChangeTransition(self.inner.get_transition())
    }

    pub fn set_transition(&mut self, v: &PyConfChangeTransition) {
        self.inner.set_transition(v.0);
    }

    // pub fn enter_joint(&self) -> PyResult<Option<bool>> {
    //     self.inner.map_as_ref(|inner| inner.enter_joint())
    // }

    // pub fn leave_joint(&self) -> PyResult<bool> {
    //     self.inner.map_as_ref(|inner| inner.leave_joint())
    // }
}
