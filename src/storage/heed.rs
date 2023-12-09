use crate::error::Result;

use heed::types::*;
use heed::{Database, Env, PolyDatabase};
use heed_traits::{BytesDecode, BytesEncode};
use parking_lot::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use raft::{prelude::*, GetEntriesContext};

use crate::config::Config;
use prost::Message;
use std::borrow::Cow;
use std::cmp::max;
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub trait LogStore: Storage {
    fn append(&mut self, entries: &[Entry]) -> Result<()>;
    fn hard_state(&self) -> Result<HardState>;
    fn set_hard_state(&mut self, hard_state: &HardState) -> Result<()>;
    fn set_hard_state_commit(&mut self, commit: u64) -> Result<()>;
    fn conf_state(&self) -> Result<ConfState>;
    fn set_conf_state(&mut self, conf_state: &ConfState) -> Result<()>;
    fn snapshot(&self, request_index: u64, to: u64) -> Result<Snapshot>;
    fn create_snapshot(&mut self, data: Vec<u8>, index: u64, term: u64) -> Result<()>;
    fn apply_snapshot(&mut self, snapshot: Snapshot) -> Result<()>;
    fn last_index(&self) -> Result<u64>;
    fn compact(&mut self, index: u64) -> Result<()>;
    fn all_entries(&self) -> raft::Result<Vec<Entry>>;
}

const SNAPSHOT_KEY: &str = "snapshot";
const LAST_INDEX_KEY: &str = "last_index";
const HARD_STATE_KEY: &str = "hard_state";
const CONF_STATE_KEY: &str = "conf_state";

macro_rules! heed_type {
    ($heed_type:ident, $type:ty) => {
        struct $heed_type;

        impl<'a> BytesEncode<'a> for $heed_type {
            type EItem = $type;
            fn bytes_encode(item: &'a Self::EItem) -> Option<Cow<'a, [u8]>> {
                let mut bytes = vec![];
                prost::Message::encode(item, &mut bytes).ok()?;
                Some(Cow::Owned(bytes))
            }
        }

        impl<'a> BytesDecode<'a> for $heed_type {
            type DItem = $type;
            fn bytes_decode(bytes: &'a [u8]) -> Option<Self::DItem> {
                prost::Message::decode(bytes).ok()
            }
        }
    };
}

#[derive(Eq, PartialEq)]
pub struct KeyString(String);

impl Ord for KeyString {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let self_num: u64 = self.0.parse().unwrap();
        let other_num: u64 = other.0.parse().unwrap();
        self_num.cmp(&other_num)
    }
}

impl PartialOrd for KeyString {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<'a> BytesEncode<'a> for KeyString {
    type EItem = String;

    fn bytes_encode(item: &'a Self::EItem) -> Option<Cow<'a, [u8]>> {
        Some(Cow::Borrowed(item.as_bytes()))
    }
}

impl<'a> BytesDecode<'a> for KeyString {
    type DItem = String;

    fn bytes_decode(bytes: &'a [u8]) -> Option<Self::DItem> {
        Some(String::from_utf8_lossy(bytes).into_owned())
    }
}

impl From<u64> for KeyString {
    fn from(num: u64) -> Self {
        KeyString(num.to_string())
    }
}

heed_type!(HeedSnapshot, Snapshot);
heed_type!(HeedEntry, Entry);
heed_type!(HeedHardState, HardState);
heed_type!(HeedConfState, ConfState);

pub struct HeedStorageCore {
    env: Env,
    entries_db: Database<KeyString, HeedEntry>,
    metadata_db: PolyDatabase,
    config: Config,
    logger: slog::Logger,
}

impl HeedStorageCore {
    pub fn create(log_dir_path: PathBuf, config: &Config, logger: slog::Logger) -> Result<Self> {
        let env = heed::EnvOpenOptions::new()
            .map_size(config.lmdb_map_size as usize)
            .max_dbs(3000)
            .open(log_dir_path)?;

        let entries_db: Database<KeyString, HeedEntry> = env.create_database(Some("entries"))?;

        let metadata_db = env.create_poly_database(Some("meta"))?;

        let hard_state = HardState::default();
        let conf_state = ConfState::default();

        let storage = Self {
            metadata_db,
            entries_db,
            env,
            logger,
            config: config.clone(),
        };

        let mut writer = storage.env.write_txn()?;
        storage.set_hard_state(&mut writer, &hard_state)?;
        storage.set_conf_state(&mut writer, &conf_state)?;
        storage.append(&mut writer, &[Entry::default()])?;
        writer.commit()?;

        Ok(storage)
    }

    fn set_hard_state(&self, writer: &mut heed::RwTxn, hard_state: &HardState) -> Result<()> {
        self.metadata_db
            .put::<_, Str, HeedHardState>(writer, HARD_STATE_KEY, hard_state)?;
        Ok(())
    }

    fn hard_state(&self, reader: &heed::RoTxn) -> Result<HardState> {
        let hard_state = self
            .metadata_db
            .get::<_, Str, HeedHardState>(reader, HARD_STATE_KEY)?;
        Ok(hard_state.expect("Missing hard_state in metadata"))
    }

    pub fn set_conf_state(&self, writer: &mut heed::RwTxn, conf_state: &ConfState) -> Result<()> {
        self.metadata_db
            .put::<_, Str, HeedConfState>(writer, CONF_STATE_KEY, conf_state)?;
        Ok(())
    }

    pub fn conf_state(&self, reader: &heed::RoTxn) -> Result<ConfState> {
        let conf_state = self
            .metadata_db
            .get::<_, Str, HeedConfState>(reader, CONF_STATE_KEY)?;
        Ok(conf_state.expect("Missing conf_state in metadata"))
    }

    fn set_snapshot(&self, writer: &mut heed::RwTxn, snapshot: &Snapshot) -> Result<()> {
        self.metadata_db
            .put::<_, Str, HeedSnapshot>(writer, SNAPSHOT_KEY, snapshot)?;
        Ok(())
    }

    pub fn snapshot(
        &self,
        reader: &heed::RoTxn,
        _request_index: u64,
        _to: u64,
    ) -> Result<Snapshot> {
        let snapshot = self
            .metadata_db
            .get::<_, Str, HeedSnapshot>(reader, SNAPSHOT_KEY)?;
        let snapshot = snapshot.unwrap_or_default();
        Ok(snapshot)
    }

    fn last_index(&self, reader: &heed::RoTxn) -> Result<u64> {
        let last_index = self
            .metadata_db
            .get::<_, Str, OwnedType<u64>>(reader, LAST_INDEX_KEY)?
            .unwrap_or(0);

        Ok(last_index)
    }

    fn set_last_index(&self, writer: &mut heed::RwTxn, index: u64) -> Result<()> {
        self.metadata_db
            .put::<_, Str, OwnedType<u64>>(writer, LAST_INDEX_KEY, &index)?;
        Ok(())
    }

    fn first_index(&self, reader: &heed::RoTxn) -> Result<u64> {
        let first_entry = self
            .entries_db
            .first(reader)?
            .expect("There should always be at least one entry in the db");

        Ok(first_entry.0.parse::<u64>().unwrap() + 1)
    }

    fn entry(&self, reader: &heed::RoTxn, index: u64) -> Result<Option<Entry>> {
        let entry = self.entries_db.get(reader, &index.to_string())?;
        Ok(entry)
    }

    fn entries(
        &self,
        reader: &heed::RoTxn,
        low: u64,
        high: u64,
        max_size: impl Into<Option<u64>>,
        _ctx: GetEntriesContext,
    ) -> Result<Vec<Entry>> {
        slog::info!(self.logger, "Entries [{}, {}) requested", low, high);

        let iter = self
            .entries_db
            .range(&reader, &(low.to_string()..high.to_string()))?;
        let max_size: Option<u64> = max_size.into();
        let mut size_count = 0;
        let mut buf = vec![];
        let mut should_not_filter = true;

        let entries = iter
            .filter_map(|e| match e {
                Ok((_, e)) => Some(e),
                _ => None,
            })
            .take_while(|entry| match max_size {
                Some(max_size) => {
                    // One entry must be returned regardless of max_size.
                    if should_not_filter {
                        should_not_filter = false;
                        return true;
                    }

                    entry.encode(&mut buf).unwrap();
                    size_count += buf.len() as u64;
                    buf.clear();
                    size_count < max_size
                }
                None => true,
            })
            .collect();
        Ok(entries)
    }

    fn all_entries(&self, reader: &heed::RoTxn) -> Result<Vec<Entry>> {
        let iter = self.entries_db.iter(&reader)?;
        let entries = iter
            .filter_map(|e| match e {
                Ok((_, e)) => Some(e),
                _ => None,
            })
            .collect();
        Ok(entries)
    }

    fn append(&self, writer: &mut heed::RwTxn, entries: &[Entry]) -> Result<()> {
        let mut last_index = self.last_index(&writer)?;
        // TODO: ensure entry arrive in the right order
        for entry in entries {
            //assert_eq!(entry.get_index(), last_index + 1);
            let index = entry.index;
            last_index = std::cmp::max(index, last_index);
            self.entries_db.put(writer, &index.to_string(), entry)?;
        }
        self.set_last_index(writer, last_index)?;
        Ok(())
    }
}

#[derive(Clone)]
pub struct HeedStorage(Arc<RwLock<HeedStorageCore>>);

impl HeedStorage {
    pub fn create(log_dir_path: PathBuf, config: &Config, logger: slog::Logger) -> Result<Self> {
        let core = HeedStorageCore::create(log_dir_path, config, logger)?;
        Ok(Self(Arc::new(RwLock::new(core))))
    }

    fn wl(&mut self) -> RwLockWriteGuard<HeedStorageCore> {
        self.0.write()
    }

    fn rl(&self) -> RwLockReadGuard<HeedStorageCore> {
        self.0.read()
    }
}

impl LogStore for HeedStorage {
    fn compact(&mut self, index: u64) -> Result<()> {
        let store = self.wl();
        let mut writer = store.env.write_txn()?;
        // TODO, check that compaction is legal
        //let last_index = self.last_index(&writer)?;
        // there should always be at least one entry in the log
        //assert!(last_index > index + 1);
        store
            .entries_db
            .delete_range(&mut writer, &(..index.to_string()))?;
        writer.commit()?;
        Ok(())
    }

    fn append(&mut self, entries: &[Entry]) -> Result<()> {
        let store = self.wl();
        let mut writer = store.env.write_txn()?;
        store.append(&mut writer, entries)?;
        writer.commit()?;
        Ok(())
    }

    fn hard_state(&self) -> Result<HardState> {
        let store = self.rl();
        let reader = store.env.read_txn()?;
        let hard_state = store.hard_state(&reader)?;
        Ok(hard_state)
    }

    fn set_hard_state(&mut self, hard_state: &HardState) -> Result<()> {
        let store = self.wl();
        let mut writer = store.env.write_txn()?;
        store.set_hard_state(&mut writer, hard_state)?;
        writer.commit()?;
        Ok(())
    }

    fn set_hard_state_commit(&mut self, commit: u64) -> Result<()> {
        let store = self.wl();
        let reader = store.env.read_txn()?;
        let mut hard_state = store.hard_state(&reader)?;
        hard_state.set_commit(commit);
        let mut writer = store.env.write_txn()?;
        store.set_hard_state(&mut writer, &hard_state)?;
        Ok(())
    }

    fn conf_state(&self) -> Result<ConfState> {
        let store = self.rl();
        let reader = store.env.read_txn()?;
        let conf_state = store.conf_state(&reader)?;
        Ok(conf_state)
    }

    fn set_conf_state(&mut self, conf_state: &ConfState) -> Result<()> {
        let store = self.wl();
        let mut writer = store.env.write_txn()?;
        store.set_conf_state(&mut writer, conf_state)?;
        writer.commit()?;
        Ok(())
    }

    fn snapshot(&self, request_index: u64, to: u64) -> Result<Snapshot> {
        let store = self.rl();
        let reader = store.env.read_txn()?;
        let snapshot = store.snapshot(&reader, request_index, to)?;
        Ok(snapshot)
    }

    fn create_snapshot(&mut self, data: Vec<u8>, index: u64, term: u64) -> Result<()> {
        let store = self.wl();
        let mut writer = store.env.write_txn()?;
        let conf_state = store.conf_state(&writer)?;

        let mut snapshot = Snapshot::default();
        snapshot.set_data(data);

        let meta = snapshot.mut_metadata();
        meta.set_conf_state(conf_state);
        meta.index = index;
        meta.term = term;

        store.set_snapshot(&mut writer, &snapshot)?;
        writer.commit()?;
        Ok(())
    }

    fn apply_snapshot(&mut self, snapshot: Snapshot) -> Result<()> {
        let store = self.wl();
        let mut writer = store.env.write_txn()?;
        let metadata = snapshot.get_metadata();
        let conf_state = metadata.get_conf_state();

        let mut hard_state = store.hard_state(&writer)?;
        hard_state.set_term(max(hard_state.term, metadata.term));
        hard_state.set_commit(metadata.index);

        store.set_hard_state(&mut writer, &hard_state)?;
        store.set_conf_state(&mut writer, conf_state)?;
        store.set_last_index(&mut writer, metadata.index)?;
        store.set_snapshot(&mut writer, &snapshot)?;
        writer.commit()?;
        Ok(())
    }

    fn last_index(&self) -> Result<u64> {
        let store = self.rl();
        let reader = store.env.read_txn()?;
        let last_index = store.last_index(&reader)?;
        Ok(last_index)
    }

    fn all_entries(&self) -> raft::Result<Vec<Entry>> {
        let store = self.rl();
        let reader = store.env.read_txn().unwrap();

        let entries = store
            .all_entries(&reader)
            .map_err(|e| raft::Error::Store(raft::StorageError::Other(e.into())))?;
        Ok(entries)
    }
}

impl Storage for HeedStorage {
    fn initial_state(&self) -> raft::Result<RaftState> {
        let store = self.rl();
        let reader = store
            .env
            .read_txn()
            .map_err(|e| raft::Error::Store(raft::StorageError::Other(e.into())))?;
        let raft_state = RaftState {
            hard_state: store
                .hard_state(&reader)
                .map_err(|e| raft::Error::Store(raft::StorageError::Other(e.into())))?,
            conf_state: store
                .conf_state(&reader)
                .map_err(|e| raft::Error::Store(raft::StorageError::Other(e.into())))?,
        };
        Ok(raft_state)
    }

    fn entries(
        &self,
        low: u64,
        high: u64,
        max_size: impl Into<Option<u64>>,
        ctx: GetEntriesContext,
    ) -> raft::Result<Vec<Entry>> {
        let store = self.rl();
        let reader = store.env.read_txn().unwrap();

        let entries = store
            .entries(&reader, low, high, max_size, ctx)
            .map_err(|e| raft::Error::Store(raft::StorageError::Other(e.into())))?;
        Ok(entries)
    }

    fn term(&self, idx: u64) -> raft::Result<u64> {
        let store = self.rl();
        let reader = store
            .env
            .read_txn()
            .map_err(|_| raft::Error::Store(raft::StorageError::Unavailable))?;
        let first_index = store
            .first_index(&reader)
            .map_err(|_| raft::Error::Store(raft::StorageError::Unavailable))?;

        let snapshot = store.snapshot(&reader, 0, 0).unwrap();
        if snapshot.get_metadata().get_index() == idx {
            return Ok(snapshot.get_metadata().get_term());
        }

        let entry = store
            .entry(&reader, idx)
            .map_err(|_| raft::Error::Store(raft::StorageError::Unavailable))?;

        match entry {
            Some(entry) => Ok(entry.term),
            None => {
                if idx < first_index {
                    return Err(raft::Error::Store(raft::StorageError::Compacted));
                }
                return Err(raft::Error::Store(raft::StorageError::Unavailable));
            }
        }
    }

    fn first_index(&self) -> raft::Result<u64> {
        let store = self.rl();
        let reader = store.env.read_txn().unwrap();
        store
            .first_index(&reader)
            .map_err(|_| raft::Error::Store(raft::StorageError::Unavailable))
    }

    fn last_index(&self) -> raft::Result<u64> {
        let store = self.rl();
        let reader = store.env.read_txn().unwrap();
        let last_index = store
            .last_index(&reader)
            .map_err(|_| raft::Error::Store(raft::StorageError::Unavailable))?;

        Ok(last_index)
    }

    fn snapshot(&self, request_index: u64, to: u64) -> raft::Result<Snapshot> {
        let store = self.rl();
        let reader = store.env.read_txn().unwrap();

        store.snapshot(&reader, request_index, to).map_err(|e| {
            log::error!("Snapshot error occurred: {:?}", e);
            raft::Error::Store(raft::StorageError::SnapshotTemporarilyUnavailable)
        })
    }
}
