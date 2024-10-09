use fs2::FileExt;
use lazy_static::lazy_static;
use std::{
    fs::{self, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    path::Path,
    sync::atomic::AtomicUsize,
};

lazy_static! {
    pub static ref COUNTER: AtomicUsize = AtomicUsize::new(1);
}

const COUNTER_FILE: &str = ".ip_counter";
const BASE_IP_PREFIX: &str = "127";
const MAX_COUNTER: usize = 256 * 256 * 254; // Total possible IP addresses

#[derive(Debug)]
pub struct TestEnvironment {
    pub loopback_address: String,
    pub base_storage_path: String,
}

pub fn get_test_environment(test_name: &str) -> TestEnvironment {
    let test_environment_path = format!("logs/{}", test_name);
    let test_environment_path = Path::new(&test_environment_path);

    // Remove if the previous test environment exists
    if test_environment_path.exists() {
        fs::remove_dir_all(test_environment_path)
            .expect("Failed to remove previous logs directory");
    }

    get_test_environment_with_counter_file(test_name, COUNTER_FILE)
}

// Function with adjustable counter file path for testing
fn get_test_environment_with_counter_file(test_name: &str, counter_file: &str) -> TestEnvironment {
    let path = Path::new(counter_file);

    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(&path)
        .expect("Failed to open IP counter file");

    // Lock the file
    file.lock_exclusive()
        .expect("Failed to lock IP counter file");

    let mut counter_str = String::new();
    file.read_to_string(&mut counter_str)
        .expect("Failed to read IP counter file");

    let mut counter: usize = if counter_str.trim().is_empty() {
        0
    } else {
        counter_str
            .trim()
            .parse()
            .expect("Invalid counter value in IP counter file")
    };

    // Wrap counter if it reaches MAX_COUNTER
    if counter >= MAX_COUNTER {
        counter = 0; // Reset counter to start from 127.0.0.1 again
    }

    // Calculate the octets
    let fourth_octet = (counter % 254) + 1; // 1 to 254
    let remaining = counter / 254;

    let third_octet = remaining % 256; // 0 to 255
    let remaining = remaining / 256;

    let second_octet = remaining % 256; // 0 to 255

    let ip_address = format!(
        "{}.{}.{}.{}",
        BASE_IP_PREFIX, second_octet, third_octet, fourth_octet
    );

    let base_storage_path = format!("./logs/{}", test_name);

    // Increment counter and wrap around if necessary
    counter = (counter + 1) % MAX_COUNTER;

    // Write the new counter back to the file
    file.set_len(0).expect("Failed to truncate IP counter file");
    file.seek(SeekFrom::Start(0))
        .expect("Failed to seek to start of IP counter file");
    file.write_all(counter.to_string().as_bytes())
        .expect("Failed to write new counter to IP counter file");

    file.unlock().expect("Failed to unlock IP counter file");

    TestEnvironment {
        loopback_address: ip_address,
        base_storage_path,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_ip_allocation_wrapping() {
        let test_name = "test_wrapping";
        let temp_dir = tempdir().expect("Failed to create temp dir");
        let counter_file_path = temp_dir.path().join(COUNTER_FILE);

        let counter_file_str = counter_file_path.to_str().unwrap();

        // Set counter to MAX_COUNTER - 1 to test wrapping
        let mut file = File::create(&counter_file_path).expect("Failed to create counter file");
        write!(file, "{}", MAX_COUNTER - 1).expect("Failed to write to counter file");

        // First call: should return last IP address 127.255.255.254
        let env = get_test_environment_with_counter_file(test_name, counter_file_str);
        assert_eq!(env.loopback_address, "127.255.255.254");

        // Second call: counter wraps around, should return first IP address 127.0.0.1
        let env = get_test_environment_with_counter_file(test_name, counter_file_str);
        assert_eq!(env.loopback_address, "127.0.0.1");
    }

    #[test]
    fn test_basic_ip_allocation() {
        let test_name = "test_basic";
        let temp_dir = tempdir().expect("Failed to create temp dir");
        let counter_file_path = temp_dir.path().join(COUNTER_FILE);

        let counter_file_str = counter_file_path.to_str().unwrap();

        // First allocation
        let env = get_test_environment_with_counter_file(test_name, counter_file_str);
        assert_eq!(env.loopback_address, "127.0.0.1");
        assert_eq!(env.base_storage_path, "./logs/test_basic");

        // Second allocation
        let env = get_test_environment_with_counter_file(test_name, counter_file_str);
        assert_eq!(env.loopback_address, "127.0.0.2");

        // Third allocation
        let env = get_test_environment_with_counter_file(test_name, counter_file_str);
        assert_eq!(env.loopback_address, "127.0.0.3");
    }

    #[test]
    fn test_ip_allocation_over_multiple_octets() {
        let test_name = "test_multiple_octets";
        let temp_dir = tempdir().expect("Failed to create temp dir");
        let counter_file_path = temp_dir.path().join(COUNTER_FILE);

        let counter_file_str = counter_file_path.to_str().unwrap();

        // Set counter to test second and third octet increments
        let mut file = File::create(&counter_file_path).expect("Failed to create counter file");
        let counter_value = (1 * 256 * 254) + (1 * 254) + (0); // Second octet = 1, third octet = 1, fourth octet = 1
        write!(file, "{}", counter_value).expect("Failed to write to counter file");

        // First call: should return IP address 127.1.1.1
        let env = get_test_environment_with_counter_file(test_name, counter_file_str);
        assert_eq!(env.loopback_address, "127.1.1.1");
    }

    #[test]
    #[should_panic(expected = "Invalid counter value in IP counter file")]
    fn test_counter_file_corrupted() {
        let test_name = "test_corrupted";
        let temp_dir = tempdir().expect("Failed to create temp dir");
        let counter_file_path = temp_dir.path().join(COUNTER_FILE);

        // Write invalid data
        fs::write(&counter_file_path, "invalid_number").expect("Failed to write to counter file");

        let counter_file_str = counter_file_path.to_str().unwrap();

        // Function call should panic
        get_test_environment_with_counter_file(test_name, counter_file_str);
    }
}
