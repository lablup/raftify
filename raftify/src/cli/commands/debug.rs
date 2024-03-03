use core::panic;
use jopemachine_raft::{logger::Slogger, Storage};
use serde_json::Value;
use std::{collections::HashMap, fs, sync::Arc};

use crate::{
    create_client,
    raft::formatter::{format_entry, format_snapshot},
    raft_node::utils::format_debugging_info,
    raft_service, Config, HeedStorage, Result, StableStorage,
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
    for entry in entries.iter() {
        println!("Key: {}, {:?}", entry.get_index(), format_entry(entry));
    }

    println!();

    println!("---- Metadata ----");
    println!("{:?}", storage.hard_state()?);
    println!("{:?}", storage.conf_state()?);
    println!("{:?}", format_snapshot(&storage.snapshot(0, 0)?));
    println!("Last index: {}", storage.last_index()?);
    Ok(())
}

pub fn debug_persitsted_all(path_str: &str, logger: slog::Logger) -> Result<()> {
    if let Ok(subdirectories) = fs::read_dir(path_str) {
        let mut dir_entries = vec![];
        for dir_entry in subdirectories.filter_map(|e| e.ok()) {
            let path = dir_entry.path();
            if path.is_dir() {
                if let Some(file_name) = path.file_name() {
                    if let Some(name) = file_name.to_str() {
                        if name.starts_with("node-") {
                            dir_entries.push(name.to_owned());
                        }
                    }
                }
            }
        }

        if dir_entries.is_empty() {
            panic!("No node directory found in {}", path_str);
        }

        dir_entries.sort();

        for name in dir_entries {
            println!("*----- {name} -----*");
            debug_persisted(&format!("{}/{}", path_str, name), logger.clone())?;
            println!();
        }
    } else {
        panic!("Invalid path: {}", path_str);
    }

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
