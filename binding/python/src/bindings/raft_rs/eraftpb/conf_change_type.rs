use pyo3::{exceptions::PyException, prelude::*, pyclass::CompareOp};
use raftify::raft::eraftpb::ConfChangeType;

#[derive(Clone)]
#[pyclass(frozen, name = "ConfChangeType")]
pub struct PyConfChangeType(pub ConfChangeType);

impl From<PyConfChangeType> for ConfChangeType {
    fn from(val: PyConfChangeType) -> Self {
        val.0
    }
}

impl From<ConfChangeType> for PyConfChangeType {
    fn from(x: ConfChangeType) -> Self {
        match x {
            ConfChangeType::AddNode => PyConfChangeType(ConfChangeType::AddNode),
            ConfChangeType::AddLearnerNode => PyConfChangeType(ConfChangeType::AddLearnerNode),
            ConfChangeType::RemoveNode => PyConfChangeType(ConfChangeType::RemoveNode),
        }
    }
}

#[pymethods]
impl PyConfChangeType {
    pub fn __richcmp__(&self, py: Python, rhs: &PyConfChangeType, op: CompareOp) -> PyObject {
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
            ConfChangeType::AddNode => "AddNode".to_string(),
            ConfChangeType::AddLearnerNode => "AddLearnerNode".to_string(),
            ConfChangeType::RemoveNode => "RemoveNode".to_string(),
        }
    }

    pub fn __int__(&self) -> u64 {
        self.0 as u64
    }

    #[staticmethod]
    pub fn from_int(v: i32, py: Python) -> PyResult<PyObject> {
        ConfChangeType::from_i32(v)
            .map(|x| PyConfChangeType(x).into_py(py))
            .ok_or_else(|| PyException::new_err("Invalid value"))
    }

    #[classattr]
    pub fn AddNode() -> Self {
        PyConfChangeType(ConfChangeType::AddNode)
    }

    #[classattr]
    pub fn AddLearnerNode() -> Self {
        PyConfChangeType(ConfChangeType::AddLearnerNode)
    }

    #[classattr]
    pub fn RemoveNode() -> Self {
        PyConfChangeType(ConfChangeType::RemoveNode)
    }
}
