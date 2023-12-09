use std::path::PathBuf;

use raftify::Config;
use raftify::HeedStorage;
use raftify::Result;
use raftify::LogStore;

pub fn debug_persisted(path: &str, logger: slog::Logger) -> Result<()> {
    let mut config = Config::default();
    config.log_dir = path.to_string();

    let path = PathBuf::from(&config.log_dir);
    let storage = HeedStorage::create(path, &config, logger.clone()).unwrap();

    let entries = storage.all_entries()?;

    println!("---- Persisted entries ----");
    for entry in entries {
        println!("{:?}", entry);
    }

    println!();

    println!("---- Metadata ----");
    println!("HardState: {:?}", storage.hard_state()?);
    println!("ConfState: {:?}", storage.conf_state()?);
    println!("Snapshot: {:?}", storage.snapshot(0, 0)?);
    println!("Last index: {:?}", storage.last_index()?);
    Ok(())
}
