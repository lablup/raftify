use pyo3::{exceptions::PyRuntimeError, prelude::*, pyclass::CompareOp};
use raftify::raft::eraftpb::EntryType;

#[derive(Clone)]
#[pyclass(frozen, name = "EntryType")]
pub struct PyEntryType(pub EntryType);

impl From<PyEntryType> for EntryType {
    fn from(val: PyEntryType) -> Self {
        val.0
    }
}

impl From<EntryType> for PyEntryType {
    fn from(x: EntryType) -> Self {
        match x {
            EntryType::EntryConfChange => PyEntryType(EntryType::EntryConfChange),
            EntryType::EntryConfChangeV2 => PyEntryType(EntryType::EntryConfChangeV2),
            EntryType::EntryNormal => PyEntryType(EntryType::EntryNormal),
        }
    }
}

#[pymethods]
impl PyEntryType {
    pub fn __richcmp__(&self, py: Python, rhs: &PyEntryType, op: CompareOp) -> PyResult<PyObject> {
        Ok(match op {
            CompareOp::Eq => (self.0 == rhs.0).into_py(py),
            CompareOp::Ne => (self.0 != rhs.0).into_py(py),
            _ => unimplemented!(),
        })
    }

    pub fn __hash__(&self) -> u64 {
        self.0 as u64
    }

    pub fn __repr__(&self) -> String {
        match self.0 {
            EntryType::EntryConfChange => "EntryConfChange".to_string(),
            EntryType::EntryConfChangeV2 => "EntryConfChangeV2".to_string(),
            EntryType::EntryNormal => "EntryNormal".to_string(),
        }
    }

    pub fn __int__(&self) -> u64 {
        self.0 as u64
    }

    #[staticmethod]
    pub fn from_int(v: i32, py: Python) -> PyResult<PyObject> {
        EntryType::from_i32(v)
            .map(|x| PyEntryType(x).into_py(py))
            .ok_or_else(|| PyRuntimeError::new_err(""))
    }

    #[classattr]
    pub fn EntryConfChange() -> Self {
        PyEntryType(EntryType::EntryConfChange)
    }

    #[classattr]
    pub fn EntryConfChangeV2() -> Self {
        PyEntryType(EntryType::EntryConfChangeV2)
    }

    #[classattr]
    pub fn EntryNormal() -> Self {
        PyEntryType(EntryType::EntryNormal)
    }
}
