use jopemachine_raft::logger::Slogger;
use serde_json::Value;
use std::{collections::HashMap, sync::Arc};

use crate::{create_client, raft_service, Config, HeedStorage, LogStore, Result};

pub async fn restore_snapshot_metadata(address: &str) -> Result<()> {
    let mut client = create_client(address).await?;
    client.create_snapshot(raft_service::Empty {}).await?;
    Ok(())
}
