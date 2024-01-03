use raft::{derializer::format_entry, eraftpb::Entry};
use serde_json::{json, Value};
use std::{
    fmt::Write as StdWrite,
    fs::{self, File, OpenOptions},
    io::{self, Read, Seek, Write as StdIoWrite},
    path::{Path, PathBuf},
};

use super::constant::ENTRY_KEY_LENGTH;
use crate::Result;

pub fn get_storage_path(log_dir: &str, node_id: u64) -> Result<PathBuf> {
    let log_dir_path = format!("{}/node-{}", log_dir, node_id);
    let log_dir_path = Path::new(&log_dir_path);

    if fs::metadata(Path::new(&log_dir_path)).is_ok() {
        fs::remove_dir_all(log_dir_path).expect("Failed to remove log directory");
    }

    fs::create_dir_all(Path::new(&log_dir_path))?;

    Ok(log_dir_path.to_path_buf())
}

pub fn format_entry_key_string(entry_key: &str) -> String {
    let entry_key: u64 = entry_key.parse().unwrap();

    let mut result = String::new();
    write!(result, "{:0width$}", entry_key, width = ENTRY_KEY_LENGTH).unwrap();
    result
}

pub fn append_to_json_file(dest_path: &Path, new_data: &[Entry]) -> io::Result<()> {
    if !dest_path.exists() {
        File::create(dest_path)?;
    }

    let mut file = OpenOptions::new().read(true).write(true).open(dest_path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;

    let mut data: Vec<Value> = if contents.is_empty() {
        Vec::new()
    } else {
        serde_json::from_str(&contents)?
    };

    for entry in new_data {
        let entry_str = format_entry(entry);
        data.push(json!(entry_str));
    }

    file.set_len(0)?;
    file.seek(io::SeekFrom::Start(0))?;
    write!(file, "{}", serde_json::to_string_pretty(&data)?)?;

    Ok(())
}
