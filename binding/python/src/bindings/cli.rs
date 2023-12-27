use pyo3::prelude::*;
use raftify::cli::cli_handler;

use super::state_machine::{PyFSM, PyLogEntry};

#[pyfunction]
pub async fn cli_main(args: Option<Vec<String>>) {
    cli_handler::<PyLogEntry, PyFSM>(args).await.unwrap();
}
