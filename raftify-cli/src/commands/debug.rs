use comfy_table::{
    presets::UTF8_FULL, Cell, CellAlignment, ContentArrangement, Table, TableComponent,
};
use core::panic;
use serde_json::Value;
use std::{
    collections::{BTreeSet, HashMap},
    fs,
    path::Path,
    sync::Arc,
};

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

pub fn debug_persisted<LogStorage: StableStorage>(
    path: &str,
    logger: slog::Logger,
    print_as_table: bool,
) -> Result<()> {
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

    if print_as_table {
        let mut table = Table::new();
        table
            .load_preset(UTF8_FULL)
            .set_content_arrangement(ContentArrangement::Dynamic);

        let formatter = CUSTOM_FORMATTER.read().unwrap();

        table
            .set_header(vec![
                Cell::new("index").set_alignment(CellAlignment::Center),
                Cell::new("type").set_alignment(CellAlignment::Center),
                Cell::new("data").set_alignment(CellAlignment::Center),
                Cell::new("term").set_alignment(CellAlignment::Center),
            ])
            .set_content_arrangement(ContentArrangement::Dynamic);

        for entry in entries.iter() {
            table.add_row(vec![
                Cell::new(entry.get_index()).set_alignment(CellAlignment::Center),
                Cell::new(
                    format!("{:?}", entry.get_entry_type())
                        .replace("Entry", "")
                        .replace("ConfChangeV2", "ConfChange"),
                )
                .set_alignment(CellAlignment::Center),
                Cell::new(formatter.format_entry_data(&entry.data.clone().into()))
                    .set_alignment(CellAlignment::Left),
                Cell::new(entry.get_term()).set_alignment(CellAlignment::Center),
            ]);
        }
        println!("{}", table);
    } else {
        for entry in entries.iter() {
            println!("Key: {}, {:?}", entry.get_index(), format_entry(entry));
        }
    }

    println!();

    println!("---- Metadata ----");
    let hard_state = storage.hard_state()?;
    let conf_state = storage.conf_state()?;
    let snapshot = storage.snapshot(0, 0)?;
    let last_index = storage.last_index()?;

    if print_as_table {
        let mut table = Table::new();
        table
            .load_preset(UTF8_FULL)
            .set_content_arrangement(ContentArrangement::Dynamic);

        table.set_header(vec![
            Cell::new("Category"),
            Cell::new("Field"),
            Cell::new("Value"),
        ]);

        table.add_row(vec![
            Cell::new("HardState"),
            Cell::new("term"),
            Cell::new(hard_state.term.to_string()),
        ]);
        table.add_row(vec![
            Cell::new(""),
            Cell::new("vote"),
            Cell::new(hard_state.vote.to_string()),
        ]);
        table.add_row(vec![
            Cell::new(""),
            Cell::new("commit"),
            Cell::new(hard_state.commit.to_string()),
        ]);

        table.add_row(vec![
            Cell::new("ConfState"),
            Cell::new("voters"),
            Cell::new(format!(
                "{:?}",
                BTreeSet::from_iter(conf_state.voters.iter().cloned())
            )),
        ]);
        table.add_row(vec![
            Cell::new(""),
            Cell::new("learners"),
            Cell::new(format!(
                "{:?}",
                BTreeSet::from_iter(conf_state.learners.iter().cloned())
            )),
        ]);
        table.add_row(vec![
            Cell::new(""),
            Cell::new("voters_outgoing"),
            Cell::new(format!(
                "{:?}",
                BTreeSet::from_iter(conf_state.voters_outgoing.iter().cloned())
            )),
        ]);
        table.add_row(vec![
            Cell::new(""),
            Cell::new("learners_next"),
            Cell::new(format!(
                "{:?}",
                BTreeSet::from_iter(conf_state.learners_next.iter().cloned())
            )),
        ]);
        table.add_row(vec![
            Cell::new(""),
            Cell::new("auto_leave"),
            Cell::new(conf_state.auto_leave.to_string()),
        ]);

        table.add_row(vec![
            Cell::new("Snapshot"),
            Cell::new("data"),
            Cell::new(format!("{:?}", snapshot.data)),
        ]);

        if let Some(metadata) = &snapshot.metadata {
            table.add_row(vec![
                Cell::new(""),
                Cell::new("metadata.index"),
                Cell::new(metadata.index.to_string()),
            ]);
            table.add_row(vec![
                Cell::new(""),
                Cell::new("metadata.term"),
                Cell::new(metadata.term.to_string()),
            ]);

            if let Some(conf_state) = &metadata.conf_state {
                table.add_row(vec![
                    Cell::new(""),
                    Cell::new("metadata.conf_state.voters"),
                    Cell::new(format!(
                        "{:?}",
                        BTreeSet::from_iter(conf_state.voters.iter().cloned())
                    )),
                ]);
                table.add_row(vec![
                    Cell::new(""),
                    Cell::new("metadata.conf_state.learners"),
                    Cell::new(format!(
                        "{:?}",
                        BTreeSet::from_iter(conf_state.learners.iter().cloned())
                    )),
                ]);
                table.add_row(vec![
                    Cell::new(""),
                    Cell::new("metadata.conf_state.voters_outgoing"),
                    Cell::new(format!(
                        "{:?}",
                        BTreeSet::from_iter(conf_state.voters_outgoing.iter().cloned())
                    )),
                ]);
                table.add_row(vec![
                    Cell::new(""),
                    Cell::new("metadata.conf_state.learners_next"),
                    Cell::new(format!(
                        "{:?}",
                        BTreeSet::from_iter(conf_state.learners_next.iter().cloned())
                    )),
                ]);
                table.add_row(vec![
                    Cell::new(""),
                    Cell::new("metadata.conf_state.auto_leave"),
                    Cell::new(conf_state.auto_leave.to_string()),
                ]);
            }
        }

        table.add_row(vec![
            Cell::new("LastIndex"),
            Cell::new("last index"),
            Cell::new(last_index.to_string()),
        ]);

        println!("{}", table);
    } else {
        println!("{:?}", storage.hard_state()?);
        println!("{:?}", storage.conf_state()?);
        println!("{:?}", format_snapshot(&storage.snapshot(0, 0)?));
        println!("Last index: {}", storage.last_index()?);
    }

    Ok(())
}

pub fn debug_persisted_all<LogStorage: StableStorage>(
    path_str: &str,
    logger: slog::Logger,
    print_as_table: bool,
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
            debug_persisted::<LogStorage>(
                &format!("{}/{}", path_str, name),
                logger.clone(),
                print_as_table,
            )?;
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
