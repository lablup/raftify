use raftify::{Result, cli::cli_handler};

use memstore::state_machine::{HashStore, LogEntry};

#[tokio::main]
async fn main() -> Result<()> {
    cli_handler::<LogEntry, HashStore>(None).await?;
    Ok(())
}
