use pyo3::{exceptions::PyException, prelude::*, pyclass::CompareOp};
use raftify::raft::eraftpb::ConfChangeTransition;

#[derive(Clone)]
#[pyclass(name = "ConfChangeTransition")]
pub struct PyConfChangeTransition(pub ConfChangeTransition);

impl From<PyConfChangeTransition> for ConfChangeTransition {
    fn from(val: PyConfChangeTransition) -> Self {
        val.0
    }
}

impl From<ConfChangeTransition> for PyConfChangeTransition {
    fn from(x: ConfChangeTransition) -> Self {
        match x {
            ConfChangeTransition::Auto => PyConfChangeTransition(ConfChangeTransition::Auto),
            ConfChangeTransition::Implicit => {
                PyConfChangeTransition(ConfChangeTransition::Implicit)
            }
            ConfChangeTransition::Explicit => {
                PyConfChangeTransition(ConfChangeTransition::Explicit)
            }
        }
    }
}

#[pymethods]
impl PyConfChangeTransition {
    pub fn __richcmp__(&self, py: Python, rhs: &PyConfChangeTransition, op: CompareOp) -> PyObject {
        match op {
            CompareOp::Eq => (self.0 == rhs.0).into_py(py),
            CompareOp::Ne => (self.0 != rhs.0).into_py(py),
            _ => unreachable!(),
        }
    }

    pub fn __hash__(&self) -> u64 {
        self.0 as u64
    }

    pub fn __repr__(&self) -> String {
        match self.0 {
            ConfChangeTransition::Auto => "Auto".to_string(),
            ConfChangeTransition::Implicit => "Implicit".to_string(),
            ConfChangeTransition::Explicit => "Explicit".to_string(),
        }
    }

    pub fn __int__(&self) -> u64 {
        self.0 as u64
    }

    #[staticmethod]
    pub fn from_int(v: i32, py: Python) -> PyResult<PyObject> {
        ConfChangeTransition::from_i32(v)
            .map(|x| PyConfChangeTransition(x).into_py(py))
            .ok_or_else(|| PyException::new_err("Invalid value"))
    }

    #[classattr]
    pub fn Auto() -> Self {
        PyConfChangeTransition(ConfChangeTransition::Auto)
    }

    #[classattr]
    pub fn Implicit() -> Self {
        PyConfChangeTransition(ConfChangeTransition::Implicit)
    }

    #[classattr]
    pub fn Explicit() -> Self {
        PyConfChangeTransition(ConfChangeTransition::Explicit)
    }
}
