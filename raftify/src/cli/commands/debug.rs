use serde_json::Value;
use std::{collections::HashMap, path::PathBuf};

use crate::create_client;
use crate::raft::deserializer::{format_entry, format_snapshot};
use crate::raft_node::utils::format_debugging_info;
use crate::raft_service;
use crate::Config;
use crate::HeedStorage;
use crate::LogStore;
use crate::Result;

pub fn debug_persisted(path: &str, logger: slog::Logger) -> Result<()> {
    let config = Config {
        log_dir: path.to_string(),
        ..Default::default()
    };

    let path = PathBuf::from(&config.log_dir);
    let storage = HeedStorage::create(path, &config, logger.clone()).unwrap();

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
    let mut client = create_client(&addr).await.unwrap();
    let response = client.debug_node(raft_service::Empty {}).await.unwrap();
    let json = response.into_inner().result;
    let parsed: HashMap<String, Value> = serde_json::from_str(&json).unwrap();

    println!("{}", format_debugging_info(&parsed));
    Ok(())
}

pub async fn debug_entries(_addr: &str) -> Result<()> {
    todo!()
}
