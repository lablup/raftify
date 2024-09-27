use raftify::{cli::cli_handler, Result};

use memstore_example_harness::state_machine::{HashStore, LogEntry};

#[cfg(feature = "inmemory_storage")]
use raftify::MemStorage as StorageType;

#[cfg(feature = "heed_storage")]
use raftify::HeedStorage as StorageType;

#[tokio::main]
async fn main() -> Result<()> {
    cli_handler::<LogEntry, StorageType, HashStore>(None).await?;

    Ok(())
}
