use pyo3::{
    prelude::*,
    types::{PyBytes, PyDict},
};
use raftify::raft::{eraftpb::Entry, formatter::format_entry};

use super::entry_type::PyEntryType;

#[derive(Clone)]
#[pyclass(name = "Entry")]
pub struct PyEntry {
    pub inner: Entry,
}

#[pymethods]
impl PyEntry {
    #[new]
    pub fn new() -> Self {
        PyEntry {
            inner: Entry::default(),
        }
    }

    pub fn __repr__(&self) -> String {
        format_entry(&self.inner)
    }

    pub fn to_dict(&mut self, py: Python) -> PyResult<PyObject> {
        let data = self.get_data(py);
        let context = self.get_context(py);
        let entry_type = self.get_entry_type().__repr__();
        let index = self.get_index();
        let term = self.get_term();
        let sync_log = self.get_sync_log();

        let res = PyDict::new(py);
        res.set_item("data", data).unwrap();
        res.set_item("context", context).unwrap();
        res.set_item("entry_type", entry_type).unwrap();
        res.set_item("index", index).unwrap();
        res.set_item("term", term).unwrap();
        res.set_item("sync_log", sync_log).unwrap();
        Ok(res.into_py(py))
    }

    pub fn get_context(&self, py: Python) -> Py<PyBytes> {
        PyBytes::new(py, self.inner.get_context()).into_py(py)
    }

    pub fn set_context(&mut self, byte_arr: &PyAny) {
        self.inner
            .set_context(byte_arr.extract::<Vec<u8>>().unwrap())
    }

    pub fn get_data(&self, py: Python) -> Py<PyBytes> {
        PyBytes::new(py, self.inner.get_data()).into_py(py)
    }

    pub fn set_data(&mut self, byte_arr: &PyAny) {
        self.inner.set_data(byte_arr.extract::<Vec<u8>>().unwrap())
    }

    pub fn get_entry_type(&self) -> PyEntryType {
        PyEntryType(self.inner.get_entry_type())
    }

    pub fn set_entry_type(&mut self, typ: &PyEntryType) {
        self.inner.set_entry_type(typ.0);
    }

    pub fn get_sync_log(&self) -> bool {
        self.inner.get_sync_log()
    }

    pub fn set_sync_log(&mut self, v: bool) {
        self.inner.set_sync_log(v)
    }

    pub fn get_term(&self) -> u64 {
        self.inner.get_term()
    }

    pub fn set_term(&mut self, v: u64) {
        self.inner.set_term(v)
    }

    pub fn get_index(&self) -> u64 {
        self.inner.get_index()
    }

    pub fn set_index(&mut self, v: u64) {
        self.inner.set_index(v)
    }
}
