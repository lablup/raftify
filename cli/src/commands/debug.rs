use std::path::PathBuf;

use raftify::Config;
use raftify::HeedStorage;
use raftify::LogStore;
use raftify::Result;

pub fn debug_persisted(path: &str, logger: slog::Logger) -> Result<()> {
    let mut config = Config::default();
    config.log_dir = path.to_string();

    let path = PathBuf::from(&config.log_dir);
    let storage = HeedStorage::create(path, &config, logger.clone()).unwrap();

    let entries = storage.all_entries()?;

    println!("---- Persisted entries ----");
    for (i, entry) in entries.iter().enumerate() {
        println!("Key {}: {:?}", i + 1, entry);
    }

    println!();

    println!("---- Metadata ----");
    println!("{:?}", storage.hard_state()?);
    println!("{:?}", storage.conf_state()?);
    println!("{:?}", storage.snapshot(0, 0)?);
    println!("Last index: {}", storage.last_index()?);
    Ok(())
}
