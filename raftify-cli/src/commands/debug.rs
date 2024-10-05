use core::panic;
use prettytable::*;
use serde_json::Value;
use std::{collections::HashMap, fs, path::Path, sync::Arc};

use raftify::{
    create_client,
    raft::{
        formatter::{format_entry, format_snapshot, CUSTOM_FORMATTER},
        logger::Slogger,
        Storage,
    },
    raft_node::utils::format_debugging_info,
    raft_service, ConfigBuilder, HeedStorage, Result, StableStorage, StorageType,
};

pub fn debug_persisted<LogStorage: StableStorage>(path: &str, logger: slog::Logger) -> Result<()> {
    let config = ConfigBuilder::new().log_dir(path.to_string()).build();

    let storage = match LogStorage::STORAGE_TYPE {
        StorageType::Heed => HeedStorage::create(
            config.get_log_dir(),
            &config,
            Arc::new(Slogger {
                slog: logger.clone(),
            }),
        )?,
        StorageType::InMemory => {
            panic!("InMemory storage does not support this feature");
        }
        _ => {
            panic!("Unsupported storage type");
        }
    };

    let entries = storage.all_entries()?;

    if !entries.is_empty() {
        assert!(
            storage.first_index()? > 0,
            "First index should be greater than 0"
        );
    }

    println!("---- Persisted entries ----");
    let mut table = Table::new();
    let formatter = CUSTOM_FORMATTER.read().unwrap();

    table.add_row(row!["index", "type", "context", "data", "term"]);

    for entry in entries.iter() {
        println!("Key: {}, {:?}", entry.get_index(), format_entry(entry));
        table.add_row(row![
            entry.get_index(),
            format!("{:?}", entry.get_entry_type()).replace("Entry", ""),
            formatter.format_entry_context(&entry.context.clone().into()),
            formatter.format_entry_data(&entry.data.clone().into()),
            entry.get_term()
        ]);
    }
    table.printstd();

    println!();

    println!("---- Metadata ----");
    println!("{:?}", storage.hard_state()?);
    println!("{:?}", storage.conf_state()?);
    println!("{:?}", format_snapshot(&storage.snapshot(0, 0)?));
    println!("Last index: {}", storage.last_index()?);
    Ok(())
}

pub fn debug_persisted_all<LogStorage: StableStorage>(
    path_str: &str,
    logger: slog::Logger,
) -> Result<()> {
    let path = match fs::canonicalize(Path::new(&path_str)) {
        Ok(absolute_path) => absolute_path,
        Err(e) => {
            panic!("Invalid path: {}", e);
        }
    };

    if let Ok(subdirectories) = fs::read_dir(path.clone()) {
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
            panic!("Raft node log directory not found: \"{}\"", path.display());
        }

        dir_entries.sort();

        for name in dir_entries {
            println!("*----- {name} -----*");
            debug_persisted::<LogStorage>(&format!("{}/{}", path_str, name), logger.clone())?;
            println!();
        }
    } else {
        panic!("Failed to read subdirectory: {}", path.display());
    }

    Ok(())
}

pub async fn debug_node(addr: &str) -> Result<()> {
    // TODO: Support TLS configuration
    let mut client = create_client(&addr, None).await?;
    let response = client.debug_node(raft_service::Empty {}).await?;
    let json = response.into_inner().result_json;
    let parsed: HashMap<String, Value> = serde_json::from_str(&json).unwrap();

    println!("{}", format_debugging_info(&parsed));
    Ok(())
}

pub async fn debug_entries(_addr: &str) -> Result<()> {
    todo!()
}
