use jopemachine_raft::logger::Slogger;
use serde_json::Value;
use std::{collections::HashMap, sync::Arc};

use crate::{
    create_client,
    raft::formatter::{format_entry, format_snapshot},
    raft_node::utils::format_debugging_info,
    raft_service, Config, HeedStorage, LogStore, Result,
};

pub fn debug_persisted(path: &str, logger: slog::Logger) -> Result<()> {
    let config = Config {
        log_dir: path.to_string(),
        ..Default::default()
    };

    let storage = HeedStorage::create(
        config.log_dir.as_str(),
        &config,
        Arc::new(Slogger {
            slog: logger.clone(),
        }),
    )?;

    let entries = storage.all_entries()?;

    println!("---- Persisted entries ----");
    for (i, entry) in entries.iter().enumerate() {
        println!("Key {}, {:?}", i + 1, format_entry(entry));
    }

    println!();

    println!("---- Metadata ----");
    println!("{:?}", storage.hard_state()?);
    println!("{:?}", storage.conf_state()?);
    println!("{:?}", format_snapshot(&storage.snapshot(0, 0)?));
    println!("Last index: {}", storage.last_index()?);
    Ok(())
}

pub async fn debug_node(addr: &str) -> Result<()> {
    let mut client = create_client(&addr).await?;
    let response = client.debug_node(raft_service::Empty {}).await?;
    let json = response.into_inner().result_json;
    let parsed: HashMap<String, Value> = serde_json::from_str(&json).unwrap();

    println!("{}", format_debugging_info(&parsed));
    Ok(())
}

pub async fn debug_entries(_addr: &str) -> Result<()> {
    todo!()
}
