use fxhash::FxHasher;
use pyo3::{
    prelude::*,
    types::{PyDict, PyString},
};
use raftify::Peers;
use std::{collections::HashMap, hash::BuildHasherDefault};

use super::{peer::PyPeer, role::PyInitialRole};

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
            .extract::<HashMap<u64, PyPeer, BuildHasherDefault<FxHasher>>>()
            .unwrap();

        let mut inner = Peers::with_empty();

        for (node_id, peer) in peers.iter() {
            inner.add_peer(*node_id, peer.inner.addr, Some(peer.inner.role.clone()));
        }

        Self { inner }
    }

    pub fn __repr__(&self) -> String {
        format!("{:?}", self.inner)
    }

    pub fn is_empty(&self) -> bool {
        self.inner.inner.is_empty()
    }

    pub fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new(py);

        for (node_id, peer) in self.inner.iter() {
            let peer = PyPeer {
                inner: peer.clone(),
            };
            dict.set_item(node_id, peer.into_py(py))?;
        }

        Ok(dict.to_object(py))
    }

    pub fn keys(&self) -> Vec<u64> {
        self.inner.iter().map(|(id, _)| id).collect()
    }

    pub fn get(&self, node_id: u64) -> Option<PyPeer> {
        self.inner.get(&node_id).map(|peer| PyPeer {
            inner: peer.clone(),
        })
    }

    pub fn add_peer(&mut self, node_id: u64, addr: &PyString, role: &PyInitialRole) {
        self.inner
            .add_peer(node_id, addr.to_str().unwrap(), Some(role.0.clone()));
    }

    pub fn remove(&mut self, node_id: u64) {
        self.inner.remove(&node_id);
    }

    pub fn get_node_id_by_addr(&mut self, addr: &PyString) -> Option<u64> {
        self.inner.get_node_id_by_addr(addr.to_str().unwrap())
    }
}
