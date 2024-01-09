use fxhash::FxHasher;
use pyo3::{
    prelude::*,
    types::{PyDict, PyString},
};
use raftify::Peers;
use std::{collections::HashMap, hash::BuildHasherDefault};

use super::utils::new_py_list;

#[derive(Clone)]
#[pyclass(dict, name = "Peers")]
pub struct PyPeers {
    pub inner: Peers,
}

#[pymethods]
impl PyPeers {
    #[new]
    pub fn new(peers: &PyDict) -> Self {
        let peers = peers
            .extract::<HashMap<u64, String, BuildHasherDefault<FxHasher>>>()
            .unwrap();

        let mut inner = Peers::with_empty();

        for (node_id, addr) in peers.iter() {
            inner.add_peer(*node_id, addr);
        }

        Self { inner }
    }

    pub fn __repr__(&self) -> PyResult<String> {
        Ok(format!("{:?}", self.inner))
    }

    pub fn is_empty(&self) -> bool {
        self.inner.inner.is_empty()
    }

    pub fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new(py);

        for (node_id, peer) in self.inner.iter() {
            dict.set_item(node_id, peer.addr.to_string())?;
        }

        Ok(dict.to_object(py))
    }

    pub fn items(&self, py: Python) -> PyResult<PyObject> {
        let peer_items = self
            .inner
            .iter()
            .map(|(id, peer)| (id, peer.addr.to_string()))
            .collect::<Vec<_>>();

        Ok(new_py_list::<(u64, String), _>(py, peer_items)?.to_object(py))
    }

    // TODO: Replace String with Peer
    pub fn get(&self, node_id: u64) -> Option<String> {
        self.inner
            .get(&node_id)
            .map(|peer| peer.addr.to_owned().to_string())
    }

    pub fn add_peer(&mut self, node_id: u64, addr: &PyString) {
        self.inner.add_peer(node_id, addr.to_str().unwrap());
    }

    pub fn remove(&mut self, node_id: u64) {
        self.inner.remove(&node_id);
    }

    pub fn get_node_id_by_addr(&mut self, addr: &PyString) -> Option<u64> {
        self.inner.get_node_id_by_addr(addr.to_str().unwrap())
    }
}
