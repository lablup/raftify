use dynamic_cluster::state_machine::{HashStore, LogEntry};
use raftify::cli::cli_handler;
use raftify::Result;

#[tokio::main]
async fn main() -> Result<()> {
    cli_handler::<LogEntry, HashStore>().await?;
    Ok(())
}
