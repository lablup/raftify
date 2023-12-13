use async_trait::async_trait;
use pyo3::{prelude::*, types::PyBytes};
use raftify::{AbstractLogEntry, AbstractStateMachine, Error};

#[derive(Clone)]
#[pyclass(name = "LogEntry")]
pub struct PyLogEntry {
    pub log_entry: Py<PyAny>,
}

impl AbstractLogEntry for PyLogEntry {
    fn encode(&self) -> Result<Vec<u8>, Error> {
        Python::with_gil(|py| {
            self.log_entry
                .as_ref(py)
                .call_method("encode", (), None)
                .and_then(|py_result| py_result.extract::<Vec<u8>>().map(|res| res))
                .map_err(|err| Error::Unknown)
        })
    }

    fn decode(data: &[u8]) -> Result<Self, Error> {
        Python::with_gil(|py| {
            let log_entry_class = PyModule::import(py, "your_python_module")
                .unwrap()
                .getattr("LogEntry")
                .unwrap();

            let py_result = log_entry_class
                .getattr("decode")
                .unwrap()
                .call1((data,))
                .unwrap();

            py_result
                .extract::<PyLogEntry>()
                .map_err(|_| Error::Unknown)
        })
    }
}

#[derive(Clone)]
#[pyclass(name = "StateMachine")]
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
impl AbstractStateMachine<PyLogEntry> for PyFSM {
    async fn apply(&mut self, log_entry: PyLogEntry) -> Result<PyLogEntry, Error> {
        Python::with_gil(|py| {
            self.store
                .as_ref(py)
                .call_method("apply", (log_entry,), None)
                .and_then(|py_result| py_result.extract::<PyLogEntry>().map(|res| res))
                .map_err(|err| Error::Unknown)
        })
    }

    async fn snapshot(&self) -> Result<PyFSM, Error> {
        Python::with_gil(|py| {
            self.store
                .as_ref(py)
                .call_method("snapshot", (), None)
                .and_then(|py_result| py_result.extract::<PyFSM>().map(|res| res))
                .map_err(|err| Error::Unknown)
        })
    }

    async fn restore(&mut self, snapshot: PyFSM) -> Result<(), Error> {
        Python::with_gil(|py| {
            self.store
                .as_ref(py)
                .call_method("restore", (snapshot,), None)
                .and_then(|_| Ok(()))
                .map_err(|err| Error::Unknown)
        })
    }

    fn encode(&self) -> Result<Vec<u8>, Error> {
        Python::with_gil(|py| {
            self.store
                .as_ref(py)
                .call_method("encode", (), None)
                .and_then(|py_result| py_result.extract::<Vec<u8>>().map(|res| res))
                .map_err(|err| Error::Unknown)
        })
    }

    fn decode(data: &[u8]) -> Result<Self, Error> {
        Python::with_gil(|py| {
            let fsm_class = PyModule::import(py, "raftify")
                .unwrap()
                .getattr("StateMachine")
                .unwrap();

            let py_result = fsm_class.getattr("decode").unwrap().call1((data,)).unwrap();

            py_result.extract::<PyFSM>().map_err(|_| Error::Unknown)
        })
    }
}
