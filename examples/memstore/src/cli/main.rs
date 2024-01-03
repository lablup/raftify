use raftify::{cli::cli_handler, Result};

use memstore::state_machine::{HashStore, LogEntry};

#[tokio::main]
async fn main() -> Result<()> {
    cli_handler::<LogEntry, HashStore>(None).await?;
    Ok(())
}
