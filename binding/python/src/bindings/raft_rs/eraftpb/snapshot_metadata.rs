use pyo3::prelude::*;

use super::conf_state::PyConfState;
use raftify::raft::eraftpb::SnapshotMetadata;

#[derive(Clone)]
#[pyclass(name = "SnapshotMetadata")]
pub struct PySnapshotMetadata {
    pub inner: SnapshotMetadata,
}

#[pymethods]
impl PySnapshotMetadata {
    #[new]
    pub fn new() -> Self {
        PySnapshotMetadata {
            inner: SnapshotMetadata::default(),
        }
    }

    pub fn get_index(&self) -> u64 {
        self.inner.get_index()
    }

    pub fn set_index(&mut self, v: u64) {
        self.inner.set_index(v)
    }

    pub fn get_term(&self) -> u64 {
        self.inner.get_term()
    }

    pub fn set_term(&mut self, v: u64) {
        self.inner.set_term(v)
    }

    pub fn get_conf_state(&mut self) -> PyConfState {
        PyConfState {
            inner: self.inner.mut_conf_state().clone(),
        }
    }

    pub fn set_conf_state(&mut self, cs: PyConfState) {
        self.inner.set_conf_state(cs.inner)
    }

    pub fn has_conf_state(&self) -> bool {
        self.inner.has_conf_state()
    }
}
