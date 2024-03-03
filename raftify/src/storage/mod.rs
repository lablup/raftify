pub mod constant;
pub mod heed_storage;
pub mod utils;

use crate::{
    error::Result,
    raft::{self, prelude::*},
};

pub trait LogStore: Storage {
    fn append(&mut self, entries: &[Entry]) -> Result<()>;
    fn hard_state(&self) -> Result<HardState>;
    fn set_hard_state(&mut self, hard_state: &HardState) -> Result<()>;
    fn set_hard_state_commit(&mut self, commit: u64) -> Result<()>;
    fn conf_state(&self) -> Result<ConfState>;
    fn set_conf_state(&mut self, conf_state: &ConfState) -> Result<()>;
    fn create_snapshot(&mut self, data: Vec<u8>, index: u64, term: u64) -> Result<()>;
    fn apply_snapshot(&mut self, snapshot: Snapshot) -> Result<()>;
    fn compact(&mut self, index: u64) -> Result<()>;
    fn all_entries(&self) -> raft::Result<Vec<Entry>>;
}
