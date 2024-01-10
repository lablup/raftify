use pyo3::{prelude::*, types::PyList};

use raftify::raft::eraftpb::ConfState;

use crate::bindings::errors::runtime_error;

#[derive(Clone)]
#[pyclass(name = "ConfState")]
pub struct PyConfState {
    pub inner: ConfState,
}

#[pymethods]
impl PyConfState {
    #[new]
    pub fn new(voters: Option<&PyList>, learners: Option<&PyList>) -> PyResult<Self> {
        if voters.and(learners).is_none() {
            Ok(PyConfState {
                inner: ConfState::default(),
            })
        } else if voters.or(learners).is_none() {
            Err(runtime_error(
                "voters and learners both values should or should not be given.",
            ))
        } else {
            let voters = voters.unwrap().extract::<Vec<u64>>()?;
            let learners = learners.unwrap().extract::<Vec<u64>>()?;
            Ok(PyConfState {
                inner: ConfState::from((voters, learners)),
            })
        }
    }

    #[staticmethod]
    pub fn default() -> Self {
        PyConfState {
            inner: ConfState::default(),
        }
    }

    pub fn get_auto_leave(&self) -> bool {
        self.inner.get_auto_leave()
    }

    pub fn set_auto_leave(&mut self, v: bool) {
        self.inner.set_auto_leave(v)
    }

    pub fn get_voters(&self, py: Python) -> PyObject {
        self.inner
            .get_voters()
            .iter()
            .map(|x| x.into_py(py))
            .collect::<Vec<_>>()
            .into_py(py)
    }

    pub fn set_voters(&mut self, list: &PyList) -> PyResult<()> {
        let v = list.extract::<Vec<u64>>()?;
        self.inner.set_voters(v);
        Ok(())
    }

    pub fn get_voters_outgoing(&self, py: Python) -> PyObject {
        self.inner
            .get_voters_outgoing()
            .iter()
            .map(|x| x.into_py(py))
            .collect::<Vec<_>>()
            .into_py(py)
    }

    pub fn set_voters_outgoing(&mut self, list: &PyList) -> PyResult<()> {
        let v = list.extract::<Vec<u64>>()?;
        self.inner.set_voters_outgoing(v);
        Ok(())
    }

    pub fn get_learners(&self, py: Python) -> PyObject {
        self.inner
            .get_learners()
            .iter()
            .map(|x| x.into_py(py))
            .collect::<Vec<_>>()
            .into_py(py)
    }

    pub fn set_learners(&mut self, list: &PyList) -> PyResult<()> {
        let v = list.extract::<Vec<u64>>()?;
        self.inner.set_learners(v);
        Ok(())
    }

    pub fn get_learners_next(&self, py: Python) -> PyObject {
        self.inner
            .get_learners_next()
            .iter()
            .map(|x| x.into_py(py))
            .collect::<Vec<_>>()
            .into_py(py)
    }

    pub fn set_learners_next(&mut self, list: &PyList) -> PyResult<()> {
        let v = list.extract::<Vec<u64>>()?;
        self.inner.set_learners_next(v);
        Ok(())
    }
}
