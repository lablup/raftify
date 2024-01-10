use pyo3::{
    prelude::*,
    types::{PyBytes, PyList},
};
use raftify::raft::{eraftpb::Message, formatter::format_message};

use super::{entry::PyEntry, message_type::PyMessageType};

#[derive(Clone)]
#[pyclass(name = "Message")]
pub struct PyMessage {
    pub inner: Message,
}

#[pymethods]
impl PyMessage {
    #[new]
    pub fn new() -> Self {
        PyMessage {
            inner: Message::default(),
        }
    }

    pub fn __repr__(&self) -> String {
        format_message(&self.inner)
    }

    pub fn get_commit(&self) -> u64 {
        self.inner.get_commit()
    }

    pub fn set_commit(&mut self, v: u64) {
        self.inner.set_commit(v)
    }

    pub fn get_commit_term(&self) -> u64 {
        self.inner.get_commit_term()
    }

    pub fn set_commit_term(&mut self, v: u64) {
        self.inner.set_commit_term(v)
    }

    pub fn get_from(&self) -> u64 {
        self.inner.get_from()
    }

    pub fn set_from(&mut self, v: u64) {
        self.inner.set_from(v)
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

    pub fn get_log_term(&self) -> u64 {
        self.inner.get_log_term()
    }

    pub fn set_log_term(&mut self, v: u64) {
        self.inner.set_log_term(v)
    }

    pub fn get_priority(&self) -> i64 {
        self.inner.get_priority()
    }

    pub fn set_priority(&mut self, v: i64) {
        self.inner.set_priority(v)
    }

    pub fn get_context(&self, py: Python) -> Py<PyBytes> {
        PyBytes::new(py, self.inner.get_context()).into_py(py)
    }

    pub fn set_context(&mut self, context: &PyAny) {
        self.inner
            .set_context(context.extract::<Vec<u8>>().unwrap());
    }

    pub fn get_reject_hint(&self) -> u64 {
        self.inner.get_reject_hint()
    }

    pub fn set_reject_hint(&mut self, v: u64) {
        self.inner.set_reject_hint(v)
    }

    pub fn get_entries(&self, py: Python) -> PyObject {
        let entries = self
            .inner
            .get_entries()
            .iter()
            .map(|entry| PyEntry {
                inner: entry.clone(),
            })
            .collect::<Vec<_>>();

        entries.into_py(py)
    }

    pub fn set_entries(&mut self, ents: &PyList) {
        let mut entries: Vec<PyEntry> = ents.extract::<Vec<_>>().unwrap();
        let entries = entries
            .iter_mut()
            .map(|x| x.inner.clone())
            .collect::<Vec<_>>();
        self.inner.set_entries(entries)
    }

    pub fn get_msg_type(&self) -> PyMessageType {
        PyMessageType(self.inner.get_msg_type())
    }

    pub fn set_msg_type(&mut self, typ: &PyMessageType) {
        self.inner.set_msg_type(typ.0)
    }

    pub fn get_reject(&self) -> bool {
        self.inner.get_reject()
    }

    pub fn set_reject(&mut self, v: bool) {
        self.inner.set_reject(v)
    }

    // pub fn get_snapshot(&mut self) -> PyResult<PySnapshotRef> {
    //     self.inner.map_as_mut(|inner| PySnapshotRef {
    //         inner: RefMutContainer::new_raw(inner.mut_snapshot()),
    //     })
    // }

    // pub fn set_snapshot(&mut self, snapshot: PySnapshotMut) -> PyResult<()> {
    //     self.inner
    //         .map_as_mut(|inner| inner.set_snapshot(snapshot.into()))
    // }

    pub fn get_to(&self) -> u64 {
        self.inner.get_to()
    }

    pub fn set_to(&mut self, v: u64) {
        self.inner.set_to(v)
    }

    pub fn get_request_snapshot(&self) -> u64 {
        self.inner.get_request_snapshot()
    }

    pub fn set_request_snapshot(&mut self, v: u64) {
        self.inner.set_request_snapshot(v)
    }

    pub fn has_snapshot(&self) -> bool {
        self.inner.has_snapshot()
    }

    #[warn(deprecated)]
    pub fn get_deprecated_priority(&self) -> u64 {
        self.inner.get_deprecated_priority()
    }

    #[warn(deprecated)]
    pub fn set_deprecated_priority(&mut self, v: u64) {
        self.inner.set_deprecated_priority(v)
    }
}
