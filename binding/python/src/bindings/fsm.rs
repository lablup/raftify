use async_trait::async_trait;
use pyo3::prelude::*;
use raftify::{AbstractStateMachine, Error};

#[derive(Clone)]
#[pyclass(name = "Storage")]
pub struct PyFSM {
    pub store: Py<PyAny>,
}

impl PyFSM {
    pub fn new(store: Py<PyAny>) -> Self {
        Self { store }
    }
}

// TODO: Handle error properly
#[async_trait]
impl AbstractStateMachine for PyFSM {
    async fn apply(&mut self, message: &[u8]) -> Result<Vec<u8>, Error> {
        Python::with_gil(|py| {
            self.store
                .as_ref(py)
                .call_method("apply", (message,), None)
                .and_then(|py_result| py_result.extract::<Vec<u8>>().map(|res| res))
                .map_err(|err| Error::Unknown)
        })
    }

    async fn snapshot(&self) -> Result<Vec<u8>, Error> {
        Python::with_gil(|py| {
            self.store
                .as_ref(py)
                .call_method("snapshot", (), None)
                .and_then(|py_result| py_result.extract::<Vec<u8>>().map(|res| res))
                .map_err(|err| Error::Unknown)
        })
    }

    async fn restore(&mut self, snapshot: &[u8]) -> Result<(), Error> {
        Python::with_gil(|py| {
            self.store
                .as_ref(py)
                .call_method("restore", (snapshot,), None)
                .and_then(|_| Ok(()))
                .map_err(|err| Error::Unknown)
        })
    }
}
