use super::constant::ENTRY_KEY_LENGTH;
use std::fmt::Write;

pub fn format_entry_key_string(entry_key: &str) -> String {
    let entry_key: u64 = entry_key.parse().unwrap();

    let mut result = String::new();
    write!(result, "{:0width$}", entry_key, width = ENTRY_KEY_LENGTH).unwrap();
    result
}
