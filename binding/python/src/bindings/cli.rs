use pyo3::prelude::*;
use pyo3_asyncio::tokio::future_into_py;
use raftify::cli::cli_handler;

use super::state_machine::{PyFSM, PyLogEntry};

// When args is None, std::env::args is automatically used.
#[pyfunction]
pub fn cli_main<'a>(args: Option<Vec<String>>, py: Python<'a>) -> PyResult<&'a PyAny> {
    future_into_py(py, async move {
        cli_handler::<PyLogEntry, PyFSM>(args).await.unwrap();
        Ok(())
    })
}
