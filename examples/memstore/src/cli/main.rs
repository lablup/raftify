use raftify::Result;
use raftify_cli::cli_handler;

use memstore_example_harness::state_machine::{HashStore, LogEntry, StorageType};

#[tokio::main]
async fn main() -> Result<()> {
    cli_handler::<LogEntry, StorageType, HashStore>(None).await?;
    Ok(())
}
