use async_trait::async_trait;
use once_cell::sync::Lazy;
use pyo3::{prelude::*, types::PyBytes};
use raftify::{AbstractLogEntry, AbstractStateMachine, Error, Result};
use std::{fmt, sync::Mutex};

use super::{
    errors::{ApplyError, RestoreError, SnapshotError},
    utils::get_python_repr,
};

pub static ENTRY_LOG_ENTRY_DESERIALIZE_CB: Lazy<Mutex<Option<PyObject>>> =
    Lazy::new(|| Mutex::new(None));
pub static ENTRY_FSM_DESERIALIZE_CB: Lazy<Mutex<Option<PyObject>>> = Lazy::new(|| Mutex::new(None));

#[pyfunction]
pub fn set_log_entry_deserializer(cb: PyObject) {
    *ENTRY_LOG_ENTRY_DESERIALIZE_CB.lock().unwrap() = Some(cb);
}

#[pyfunction]
pub fn set_fsm_deserializer(cb: PyObject) {
    *ENTRY_FSM_DESERIALIZE_CB.lock().unwrap() = Some(cb);
}

#[derive(Clone)]
#[pyclass]
pub struct PyLogEntry {
    pub log_entry: Py<PyAny>,
}

impl fmt::Debug for PyLogEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Python::with_gil(|py| {
            write!(
                f,
                "{}",
                format!("{}", get_python_repr(self.log_entry.as_ref(py)))
            )
        })
    }
}

impl fmt::Display for PyLogEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Python::with_gil(|py| {
            write!(
                f,
                "{}",
                format!("{}", get_python_repr(self.log_entry.as_ref(py)))
            )
        })
    }
}

impl AbstractLogEntry for PyLogEntry {
    fn encode(&self) -> Result<Vec<u8>> {
        Python::with_gil(|py| {
            self.log_entry
                .as_ref(py)
                .call_method("encode", (), None)
                .and_then(|py_result| py_result.extract::<Vec<u8>>().map(|res| res))
                .map_err(|err| Error::EncodingError(err.to_string()))
        })
    }

    fn decode(data: &[u8]) -> Result<Self> {
        Python::with_gil(|py| {
            let callback_lock = ENTRY_LOG_ENTRY_DESERIALIZE_CB.lock().unwrap();

            if let Some(callback) = &*callback_lock {
                let inner = callback.call(py, (data,), None).unwrap();
                Ok(PyLogEntry { log_entry: inner })
            } else {
                Err(Error::DecodingError(
                    "No deserializer for LogEntry specified".to_string(),
                ))
            }
        })
    }
}

#[pymethods]
impl PyLogEntry {
    fn __getattr__(&self, py: Python, attr: &str) -> PyResult<PyObject> {
        let log_entry: &PyAny = self.log_entry.as_ref(py);
        let attr_value = log_entry.getattr(attr)?;
        Ok(Py::from(attr_value))
    }
}

#[derive(Clone)]
#[pyclass]
pub struct PyFSM {
    pub store: Py<PyAny>,
}

impl fmt::Debug for PyFSM {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Python::with_gil(|py| {
            write!(
                f,
                "{}",
                format!("{}", get_python_repr(self.store.as_ref(py)))
            )
        })
    }
}

impl fmt::Display for PyFSM {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Python::with_gil(|py| {
            write!(
                f,
                "{}",
                format!("{}", get_python_repr(self.store.as_ref(py)))
            )
        })
    }
}

#[pymethods]
impl PyFSM {
    #[new]
    pub fn new(store: Py<PyAny>) -> Self {
        Self { store }
    }

    fn __getattr__(&self, py: Python, attr: &str) -> PyResult<PyObject> {
        let store: &PyAny = self.store.as_ref(py);
        let attr_value = store.getattr(attr)?;
        Ok(Py::from(attr_value))
    }
}

#[async_trait]
impl AbstractStateMachine for PyFSM {
    async fn apply(&mut self, log_entry: Vec<u8>) -> Result<Vec<u8>> {
        Python::with_gil(|py| {
            self.store
                .as_ref(py)
                .call_method("apply", (PyBytes::new(py, log_entry.as_slice()),), None)
                .and_then(|py_result| py_result.extract::<Vec<u8>>().map(|res| res))
                .map_err(|err| Error::Other(Box::new(ApplyError::new_err(err.to_string()))))
        })
    }

    async fn snapshot(&self) -> Result<Vec<u8>> {
        Python::with_gil(|py| {
            // TODO: Make snapshot method call to async if possible
            self.store
                .as_ref(py)
                .call_method("snapshot", (), None)
                .and_then(|py_result| py_result.extract::<Vec<u8>>().map(|res| res))
                .map_err(|err| Error::Other(Box::new(SnapshotError::new_err(err.to_string()))))
        })
    }

    async fn restore(&mut self, snapshot: Vec<u8>) -> Result<()> {
        Python::with_gil(|py| {
            self.store
                .as_ref(py)
                .call_method("restore", (PyBytes::new(py, snapshot.as_slice()),), None)
                .and_then(|_| Ok(()))
                .map_err(|err| Error::Other(Box::new(RestoreError::new_err(err.to_string()))))
        })
    }

    fn encode(&self) -> Result<Vec<u8>> {
        Python::with_gil(|py| {
            self.store
                .as_ref(py)
                .call_method("encode", (), None)
                .and_then(|py_result| py_result.extract::<Vec<u8>>().map(|res| res))
                .map_err(|err| Error::EncodingError(err.to_string()))
        })
    }

    fn decode(data: &[u8]) -> Result<Self> {
        Python::with_gil(|py| {
            let callback_lock = ENTRY_FSM_DESERIALIZE_CB.lock().unwrap();

            match &*callback_lock {
                Some(callback) => {
                    let inner = callback.call(py, (data,), None).unwrap();
                    Ok(PyFSM { store: inner })
                }
                None => Err(Error::DecodingError(
                    "No deserializer for AbstractStateMachine specified".to_string(),
                )),
            }
        })
    }
}
