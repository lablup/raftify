use chrono::Utc;
use serde_json::{json, Value};
use std::{
    fs::{self, File, OpenOptions},
    io::{self, Read, Seek, Write as StdIoWrite},
    path::Path,
};

use crate::{
    raft::{eraftpb::Entry, formatter::Bytes, formatter::CUSTOM_FORMATTER},
    Result,
};

pub fn clear_storage_path(log_dir_path: &str) -> Result<()> {
    let log_dir_path = Path::new(&log_dir_path);

    if fs::metadata(log_dir_path).is_ok() {
        fs::remove_dir_all(log_dir_path)?;
    }

    Ok(())
}

pub fn ensure_directory_exist(dir_pth: &str) -> Result<()> {
    let dir_pth: &Path = Path::new(&dir_pth);

    if fs::metadata(dir_pth).is_err() {
        fs::create_dir_all(dir_pth)?;
    }
    Ok(())
}

fn entry_type_to_str(entry_type: i32) -> &'static str {
    match entry_type {
        0 => "EntryNormal",
        1 => "EntryConfChange",
        2 => "EntryConfChangeV2",
        _ => unreachable!(),
    }
}

pub fn append_compacted_logs(dest_path: &Path, new_data: &[Entry]) -> io::Result<()> {
    if new_data.is_empty() {
        return Ok(());
    }

    if !dest_path.exists() {
        File::create(dest_path)?;
    }

    let mut file = OpenOptions::new().read(true).write(true).open(dest_path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;

    let mut existing_data: Vec<Value> = if contents.is_empty() {
        Vec::new()
    } else {
        serde_json::from_str(&contents)?
    };

    let formatter = CUSTOM_FORMATTER.read().unwrap();

    let mut new_data_json = Vec::new();
    for entry in new_data {
        new_data_json.push(json!({
            "entry_type": entry_type_to_str(entry.entry_type),
            "term": entry.term,
            "index": entry.index,
            "data": formatter.format_entry_data(&Bytes::Prost(entry.data.clone())),
            "context": formatter.format_entry_context(&Bytes::Prost(entry.context.clone())),
            "sync_log": entry.sync_log,
        }));
    }

    let timestamp = Utc::now().to_rfc3339();

    existing_data.push(json!({ timestamp: new_data_json }));

    file.set_len(0)?;
    file.seek(io::SeekFrom::Start(0))?;
    write!(file, "{}", serde_json::to_string_pretty(&existing_data)?)?;

    Ok(())
}
