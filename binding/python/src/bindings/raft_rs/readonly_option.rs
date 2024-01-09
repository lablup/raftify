use pyo3::{prelude::*, pyclass::CompareOp};
use raftify::raft::ReadOnlyOption;

#[derive(Clone)]
#[pyclass(name = "ReadOnlyOption")]
pub struct PyReadOnlyOption(pub ReadOnlyOption);

impl From<PyReadOnlyOption> for ReadOnlyOption {
    fn from(val: PyReadOnlyOption) -> Self {
        val.0
    }
}

impl From<ReadOnlyOption> for PyReadOnlyOption {
    fn from(x: ReadOnlyOption) -> Self {
        match x {
            ReadOnlyOption::Safe => PyReadOnlyOption(ReadOnlyOption::Safe),
            ReadOnlyOption::LeaseBased => PyReadOnlyOption(ReadOnlyOption::LeaseBased),
        }
    }
}

#[pymethods]
impl PyReadOnlyOption {
    pub fn __richcmp__(&self, py: Python, rhs: &PyReadOnlyOption, op: CompareOp) -> PyObject {
        match op {
            CompareOp::Eq => (self.0 == rhs.0).into_py(py),
            CompareOp::Ne => (self.0 != rhs.0).into_py(py),
            _ => unimplemented!(),
        }
    }

    pub fn __hash__(&self) -> u64 {
        self.0 as u64
    }

    pub fn __repr__(&self) -> String {
        match self.0 {
            ReadOnlyOption::Safe => "Safe".to_string(),
            ReadOnlyOption::LeaseBased => "LeaseBased".to_string(),
        }
    }

    #[classattr]
    pub fn Safe() -> Self {
        PyReadOnlyOption(ReadOnlyOption::Safe)
    }

    #[classattr]
    pub fn LeaseBased() -> Self {
        PyReadOnlyOption(ReadOnlyOption::LeaseBased)
    }
}
