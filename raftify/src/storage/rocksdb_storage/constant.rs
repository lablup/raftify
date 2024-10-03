pub const SNAPSHOT_KEY: &[u8] = b"snapshot";
pub const LAST_INDEX_KEY: &[u8] = b"last_index";
pub const HARD_STATE_KEY: &[u8] = b"hard_state";
pub const CONF_STATE_KEY: &[u8] = b"conf_state";

pub const ENTRY_KEY_LENGTH: usize = 10;

pub const METADATA_CF_KEY: &str = "meta_data";
pub const LOG_ENTRY_CF_KEY: &str = "log_entries";
