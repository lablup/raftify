use pyo3::prelude::*;

mod bindings;

#[pymodule]
fn raftify(py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<bindings::config::PyConfig>()?;
    m.add_class::<bindings::fsm::PyFSM>()?;
    m.add_class::<bindings::raft_facade::RaftFacade>()?;
    Ok(())
}
