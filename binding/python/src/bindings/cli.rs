use lazy_static::lazy_static;
use pyo3::prelude::*;
use raftify::cli::cli_handler;
use tokio::runtime::Runtime;

use super::state_machine::{PyFSM, PyLogEntry};

lazy_static! {
    static ref TOKIO_CLI_RT: Runtime = Runtime::new().unwrap();
}

// When args is None, std::env::args is automatically used.
#[pyfunction]
pub async fn cli_main(args: Option<Vec<String>>) {
    TOKIO_CLI_RT
        .spawn(cli_handler::<PyLogEntry, PyFSM>(args))
        .await
        .unwrap()
        .unwrap();
}
