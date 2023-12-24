// #![cfg(feature = "include-python-workspace")]
use pyo3::prelude::*;

mod bindings;

#[pymodule]
fn raftify(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<bindings::config::PyConfig>()?;
    m.add_class::<bindings::raft_rs::config::PyRaftConfig>()?;
    m.add_class::<bindings::fsm::PyFSM>()?;
    m.add_class::<bindings::raft_facade::PyRaftFacade>()?;
    m.add_class::<bindings::peers::PyPeers>()?;
    m.add_class::<bindings::raft_client::PyRaftClient>()?;
    m.add_class::<bindings::raft_node::PyRaftNode>()?;
    Ok(())
}
