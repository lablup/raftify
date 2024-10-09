use fs2::FileExt;
use lazy_static::lazy_static;
use std::{
    fs::OpenOptions,
    io::{Read, Seek, SeekFrom, Write},
    path::Path,
    sync::atomic::AtomicUsize,
};

use crate::cleaner::register_cleaner;

lazy_static! {
    pub static ref COUNTER: AtomicUsize = AtomicUsize::new(1);
}

const COUNTER_FILE: &str = ".ip_counter";
const BASE_IP_PREFIX: &str = "127.0.";
const MAX_COUNTER: usize = 256 * 254;

#[derive(Debug)]
pub struct TestEnvironment {
    pub loopback_address: String,
    pub base_storage_path: String,
}

pub fn get_test_environment(test_name: &str) -> TestEnvironment {
    register_cleaner();

    let path = Path::new(COUNTER_FILE);
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(&path)
        .expect("Failed to open IP counter file");

    file.lock_exclusive()
        .expect("Failed to lock IP counter file");

    let mut counter_str = String::new();
    file.read_to_string(&mut counter_str)
        .expect("Failed to read IP counter file");

    let counter: usize = if counter_str.trim().is_empty() {
        0
    } else {
        counter_str
            .trim()
            .parse()
            .expect("Invalid counter value in IP counter file")
    };

    let fourth_octet = (counter % 254) + 1; // 1..=254
    let third_octet = (counter / 254) % 256; // 0..=255

    if third_octet > 255 {
        panic!("IP address range exceeded");
    }

    let ip_address = format!("{}{}.{}", BASE_IP_PREFIX, third_octet, fourth_octet);
    let base_storage_path = format!("./logs/{}", test_name);

    let new_counter = if counter + 1 >= MAX_COUNTER {
        0
    } else {
        counter + 1
    };

    file.set_len(0).expect("Failed to truncate IP counter file");
    file.seek(SeekFrom::Start(0))
        .expect("Failed to seek to start of IP counter file");
    file.write_all(new_counter.to_string().as_bytes())
        .expect("Failed to write new counter to IP counter file");

    file.unlock().expect("Failed to unlock IP counter file");

    TestEnvironment {
        loopback_address: ip_address,
        base_storage_path,
    }
}
