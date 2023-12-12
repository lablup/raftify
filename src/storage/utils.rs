use crate::Result;
use std::{
    fs,
    path::{Path, PathBuf},
};

pub fn get_storage_path(log_dir: &str, node_id: u64) -> Result<PathBuf> {
    let log_dir_path = format!("{}/node-{}", log_dir, node_id);
    let log_dir_path = Path::new(&log_dir_path);

    if fs::metadata(Path::new(&log_dir_path)).is_ok() {
        fs::remove_dir_all(log_dir_path).expect("Failed to remove log directory");
    }

    fs::create_dir_all(Path::new(&log_dir_path))?;

    Ok(log_dir_path.to_path_buf())
}
