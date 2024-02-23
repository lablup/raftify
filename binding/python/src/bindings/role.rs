use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::pyclass::CompareOp;
use raftify::InitialRole;

#[pyclass(name = "InitialRole")]
#[derive(PartialEq, Eq, Clone, Debug)]
pub struct PyInitialRole(pub InitialRole);

#[pymethods]
impl PyInitialRole {
    #[classattr]
    const LEADER: Self = Self(InitialRole::Leader);

    #[classattr]
    const VOTER: Self = Self(InitialRole::Voter);

    #[classattr]
    const LEARNER: Self = Self(InitialRole::Learner);

    pub fn __repr__(&self) -> String {
        match self.0 {
            InitialRole::Leader => "Leader".to_string(),
            InitialRole::Voter => "Voter".to_string(),
            InitialRole::Learner => "Learner".to_string(),
        }
    }

    pub fn __int__(&self) -> u64 {
        self.0.clone() as u64
    }

    #[staticmethod]
    pub fn from_str(role: String) -> PyResult<Self> {
        match role.to_lowercase().as_str() {
            "leader" => Ok(Self(InitialRole::Leader)),
            "voter" => Ok(Self(InitialRole::Voter)),
            "learner" => Ok(Self(InitialRole::Learner)),
            _ => Err(PyValueError::new_err("Invalid role")),
        }
    }

    pub fn __richcmp__(&self, py: Python, rhs: &PyInitialRole, op: CompareOp) -> PyObject {
        match op {
            CompareOp::Eq => (self.0 == rhs.0).into_py(py),
            CompareOp::Ne => (self.0 != rhs.0).into_py(py),
            _ => py.NotImplemented(),
        }
    }
}
