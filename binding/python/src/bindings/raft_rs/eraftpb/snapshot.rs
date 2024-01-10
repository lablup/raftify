use pyo3::{prelude::*, types::PyBytes};

use raftify::raft::eraftpb::Snapshot;

use super::snapshot_metadata::PySnapshotMetadata;

#[derive(Clone)]
#[pyclass(name = "Snapshot")]
pub struct PySnapshot {
    pub inner: Snapshot,
}

#[pymethods]
impl PySnapshot {
    #[new]
    pub fn new() -> Self {
        PySnapshot {
            inner: Snapshot::default(),
        }
    }

    pub fn __bool__(&self) -> bool {
        !self.inner.is_empty()
    }

    pub fn get_data(&self, py: Python) -> Py<PyBytes> {
        PyBytes::new(py, self.inner.get_data()).into_py(py)
    }

    pub fn set_data(&mut self, bytes: &PyAny) -> PyResult<()> {
        let bytes = bytes.extract::<Vec<u8>>()?;
        Ok(self.inner.set_data(bytes))
    }

    pub fn get_metadata(&mut self) -> PySnapshotMetadata {
        PySnapshotMetadata {
            inner: self.inner.mut_metadata().clone(),
        }
    }

    pub fn set_metadata(&mut self, snapshot_meta_data: PySnapshotMetadata) {
        self.inner.set_metadata(snapshot_meta_data.inner)
    }

    pub fn has_metadata(&self) -> bool {
        self.inner.has_metadata()
    }
}
