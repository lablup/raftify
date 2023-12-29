use std::{
    fmt::Write,
    fs,
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
