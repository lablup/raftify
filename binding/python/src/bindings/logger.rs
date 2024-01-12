use pyo3::prelude::*;
use raftify::raft::logger::Logger;

#[derive(Clone, Debug)]
#[pyclass(name = "Logger")]
pub struct PyLogger {
    pub logger: Py<PyAny>,
}

impl Logger for PyLogger {
    fn info(&self, s: &str) {
        Python::with_gil(|py| {
            self.logger.call_method1(py, "info", (s,)).unwrap();
        })
    }

    fn debug(&self, s: &str) {
        Python::with_gil(|py| {
            self.logger.call_method1(py, "debug", (s,)).unwrap();
        })
    }

    fn trace(&self, s: &str) {
        Python::with_gil(|py| {
            self.logger.call_method1(py, "trace", (s,)).unwrap();
        })
    }

    fn error(&self, s: &str) {
        Python::with_gil(|py| {
            self.logger.call_method1(py, "error", (s,)).unwrap();
        })
    }

    fn warn(&self, s: &str) {
        Python::with_gil(|py| {
            self.logger.call_method1(py, "warn", (s,)).unwrap();
        })
    }

    fn fatal(&self, s: &str) {
        Python::with_gil(|py| {
            self.logger.call_method1(py, "fatal", (s,)).unwrap();
        })
    }
}

#[pymethods]
impl PyLogger {
    #[new]
    pub fn new(logger: Py<PyAny>) -> Self {
        Self { logger }
    }
}
