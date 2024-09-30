use raftify::Result;
use std::{fs, path::Path};

pub fn get_storage_path(log_dir: &str, node_id: u64) -> String {
    format!("{}/node-{}", log_dir, node_id)
}

pub fn get_data_mdb_path(log_dir: &str, node_id: u64) -> String {
    format!("{}/data.mdb", get_storage_path(log_dir, node_id))
}

pub fn ensure_directory_exist(dir_pth: &str) -> Result<()> {
    let dir_pth: &Path = Path::new(&dir_pth);

    if fs::metadata(dir_pth).is_err() {
        fs::create_dir_all(dir_pth)?;
    }
    Ok(())
}
