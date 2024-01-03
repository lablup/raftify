use raft::eraftpb::{ConfChange, ConfChangeSingle, ConfChangeV2};
use serde_json::json;
use std::fs;
use std::io::Write;
use std::path::Path;
use tokio::sync::Mutex;

pub fn to_confchange_v2(conf_change: ConfChange) -> ConfChangeV2 {
    let mut cc_v2 = ConfChangeV2::default();

    let mut cs = ConfChangeSingle::default();
    cs.set_node_id(conf_change.node_id);
    cs.set_change_type(conf_change.get_change_type());
    cc_v2.set_changes(vec![cs]);
    cc_v2.set_context(conf_change.context);

    cc_v2
}

pub fn get_filesize(path: &str) -> u64 {
    match fs::metadata(path) {
        Ok(metadata) => metadata.len(),
        Err(_) => 0,
    }
}

pub struct OneShotMutex<T> {
    data: Mutex<Option<T>>,
}

impl<T> OneShotMutex<T> {
    pub fn new(data: T) -> Self {
        OneShotMutex {
            data: Mutex::new(Some(data)),
        }
    }

    pub async fn lock(&self) -> Option<T> {
        let mut data = self.data.lock().await;
        data.take()
    }
}

#[macro_export]
macro_rules! function_name {
    () => {{
        fn f() {}
        fn type_name_of<T>(_: T) -> &'static str {
            std::any::type_name::<T>()
        }
        let name = type_name_of(f);
        &name[..name.len() - 3] // subtract length of "f"
    }};
}

// pub fn append_to_json_file(dest_path: &str, new_data: &Vec<Entry>) {
//     let path = Path::new(dest_path);
//     fs::create_dir_all(path.parent().unwrap()).unwrap();

//     let mut data = match fs::read_to_string(path) {
//         Ok(file) => {
//             let mut json: Vec<RequestIdResponse> = serde_json::from_str(&file).unwrap();
//             json.extend(new_data.clone());
//             json
//         },
//         Err(_) => new_data.clone(),
//     };

//     let file = fs::File::create(path).unwrap();
//     let mut file = std::io::BufWriter::new(file);
//     write!(file, "{}", json!(data)).unwrap();
// }
