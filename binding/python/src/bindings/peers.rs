use std::{collections::HashMap, hash::BuildHasherDefault};

use fxhash::FxHasher;
use pyo3::{
    prelude::*,
    types::{PyDict, PyString},
};
use raftify::Peers;

#[derive(Clone)]
#[pyclass(name = "Peers")]
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

        let mut inner = Peers::new();

        for (node_id, addr) in peers.iter() {
            inner.add_peer(*node_id, addr);
        }

        Self { inner }
    }

    pub fn __repr__(&self) -> PyResult<String> {
        Ok(format!("{:?}", self.inner))
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
