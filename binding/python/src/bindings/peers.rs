use std::{collections::HashMap, hash::BuildHasherDefault};

use fxhash::FxHasher;
use pyo3::{prelude::*, types::PyDict};
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
}
