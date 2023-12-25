#![cfg(feature = "include-python-workspace")]
use raftify::{raft::derializer::set_custom_deserializer, MyDeserializer};
use bindings::state_machine::{PyFSM, PyLogEntry};
use pyo3::prelude::*;

mod bindings;

#[pymodule]
fn raftify(_py: Python, m: &PyModule) -> PyResult<()> {
    // TODO: How to handle this?
    // set_custom_deserializer(MyDeserializer::<PyLogEntry, PyFSM>::new());

    m.add_class::<bindings::config::PyConfig>()?;
    m.add_class::<bindings::raft_rs::config::PyRaftConfig>()?;
    m.add_class::<bindings::state_machine::PyFSM>()?;
    m.add_class::<bindings::raft_facade::PyRaftFacade>()?;
    m.add_class::<bindings::peers::PyPeers>()?;
    m.add_class::<bindings::raft_client::PyRaftClient>()?;
    m.add_class::<bindings::raft_node::PyRaftNode>()?;
    Ok(())
}
