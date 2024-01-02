use bincode::{deserialize, serialize};
use heed::types::{Bytes as HeedBytes, Str as HeedStr};
use heed::{Database, Env};
use heed_traits::{BoxedError, BytesDecode, BytesEncode};
use parking_lot::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use prost::Message as PMessage;
use raft::{prelude::*, GetEntriesContext};
use std::borrow::Cow;
use std::cmp::max;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::{fmt, fs};

use super::constant::{CONF_STATE_KEY, HARD_STATE_KEY, LAST_INDEX_KEY, SNAPSHOT_KEY};
use super::utils::{append_to_json_file, format_entry_key_string};
use crate::config::Config;
use crate::error::Result;

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

#[derive(Eq, PartialEq)]
pub struct HeedEntryKeyString(String);

impl<'a> BytesEncode<'a> for HeedEntryKeyString {
    type EItem = String;

    fn bytes_encode(item: &'a Self::EItem) -> std::result::Result<Cow<'a, [u8]>, BoxedError> {
        Ok(Cow::Owned(
            format_entry_key_string(item).as_bytes().to_vec(),
        ))
    }
}

impl<'a> BytesDecode<'a> for HeedEntryKeyString {
    type DItem = String;

    fn bytes_decode(bytes: &'a [u8]) -> std::result::Result<Self::DItem, BoxedError> {
        Ok(String::from_utf8_lossy(bytes).into_owned())
    }
}

enum HeedEntry {}

impl BytesEncode<'_> for HeedEntry {
    type EItem = Entry;

    fn bytes_encode(item: &Self::EItem) -> std::result::Result<Cow<'_, [u8]>, BoxedError> {
        let mut bytes = vec![];
        item.encode(&mut bytes)?;
        Ok(Cow::Owned(bytes))
    }
}

impl BytesDecode<'_> for HeedEntry {
    type DItem = Entry;

    fn bytes_decode(bytes: &[u8]) -> std::result::Result<Self::DItem, BoxedError> {
        Ok(Entry::decode(bytes)?)
    }
}

#[derive(Clone)]
pub struct HeedStorage(Arc<RwLock<HeedStorageCore>>);

// TODO: Improve this.
impl fmt::Debug for HeedStorage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("HeedStorage")
            .field(&format_args!("..."))
            .finish()
    }
}

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
        let reader = store.env.read_txn()?;
        let mut writer = store.env.write_txn()?;
        store.compact(&reader, &mut writer, index)?;
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
                    Err(raft::Error::Store(raft::StorageError::Compacted))
                } else {
                    Err(raft::Error::Store(raft::StorageError::Unavailable))
                }
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

        store
            .snapshot(&reader, request_index, to)
            .map_err(|_| raft::Error::Store(raft::StorageError::SnapshotTemporarilyUnavailable))
    }
}

pub struct HeedStorageCore {
    env: Env,
    entries_db: Database<HeedEntryKeyString, HeedEntry>,
    metadata_db: Database<HeedStr, HeedBytes>,
    #[allow(dead_code)]
    config: Config,
    logger: slog::Logger,
}

impl HeedStorageCore {
    pub fn create(log_dir_path: PathBuf, config: &Config, logger: slog::Logger) -> Result<Self> {
        let env = heed::EnvOpenOptions::new()
            .map_size(config.lmdb_map_size as usize)
            .max_dbs(3000)
            .open(log_dir_path)?;

        let mut writer = env.write_txn()?;

        let entries_db: Database<HeedEntryKeyString, HeedEntry> =
            env.create_database(&mut writer, Some("entries"))?;
        let metadata_db: Database<HeedStr, HeedBytes> =
            env.create_database(&mut writer, Some("meta"))?;

        writer.commit()?;

        let storage = Self {
            metadata_db,
            entries_db,
            env,
            logger,
            config: config.clone(),
        };

        let mut writer = storage.env.write_txn()?;
        storage.append(&mut writer, &[Entry::default()])?;
        writer.commit()?;

        Ok(storage)
    }

    pub fn compact(
        &self,
        reader: &heed::RoTxn,
        writer: &mut heed::RwTxn,
        index: u64,
    ) -> Result<()> {
        // TODO, check that compaction is legal
        //let last_index = self.last_index(&writer)?;
        // there should always be at least one entry in the log
        //assert!(last_index > index + 1);

        let index = format_entry_key_string(index.to_string().as_str());

        if self.config.save_compacted_logs {
            let iter = self.entries_db.range(reader, &(..index.clone()))?;

            let entries = iter
                .filter_map(|e| match e {
                    Ok((_, e)) => Some(e),
                    _ => None,
                })
                .collect::<Vec<Entry>>();

            self.save_compacted_entries(entries.as_slice())?;
        }

        self.entries_db.delete_range(writer, &(..index.clone()))?;
        Ok(())
    }

    fn set_hard_state(&self, writer: &mut heed::RwTxn, hard_state: &HardState) -> Result<()> {
        self.metadata_db.put(
            writer,
            HARD_STATE_KEY,
            hard_state.encode_to_vec().as_slice(),
        )?;

        Ok(())
    }

    fn hard_state(&self, reader: &heed::RoTxn) -> Result<HardState> {
        let hard_state = self.metadata_db.get(reader, HARD_STATE_KEY)?;

        match hard_state {
            Some(hard_state) => {
                let hard_state = HardState::decode(hard_state)?;
                Ok(hard_state)
            }
            None => Ok(HardState::default()),
        }
    }

    pub fn set_conf_state(&self, writer: &mut heed::RwTxn, conf_state: &ConfState) -> Result<()> {
        self.metadata_db.put(
            writer,
            CONF_STATE_KEY,
            conf_state.encode_to_vec().as_slice(),
        )?;
        Ok(())
    }

    pub fn conf_state(&self, reader: &heed::RoTxn) -> Result<ConfState> {
        let conf_state = self.metadata_db.get(reader, CONF_STATE_KEY)?;

        match conf_state {
            Some(conf_state) => {
                let conf_state = ConfState::decode(conf_state)?;
                Ok(conf_state)
            }
            None => Ok(ConfState::default()),
        }
    }

    fn set_snapshot(&self, writer: &mut heed::RwTxn, snapshot: &Snapshot) -> Result<()> {
        self.metadata_db
            .put(writer, SNAPSHOT_KEY, snapshot.encode_to_vec().as_slice())?;
        Ok(())
    }

    pub fn snapshot(
        &self,
        reader: &heed::RoTxn,
        _request_index: u64,
        _to: u64,
    ) -> Result<Snapshot> {
        let snapshot = self.metadata_db.get(reader, SNAPSHOT_KEY)?;

        Ok(match snapshot {
            Some(snapshot) => Snapshot::decode(snapshot)?,
            None => Snapshot::default(),
        })
    }

    fn last_index(&self, reader: &heed::RoTxn) -> Result<u64> {
        let last_index = self.metadata_db.get(reader, LAST_INDEX_KEY)?;

        match last_index {
            Some(last_index) => {
                let last_index: u64 = deserialize(last_index)?;
                Ok(last_index)
            }
            None => Ok(0),
        }
    }

    fn set_last_index(&self, writer: &mut heed::RwTxn, index: u64) -> Result<()> {
        self.metadata_db
            .put(writer, LAST_INDEX_KEY, serialize(&index)?.as_slice())?;
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

        let low_str = format_entry_key_string(low.to_string().as_str());
        let high_str = format_entry_key_string(high.to_string().as_str());

        let iter = self.entries_db.range(reader, &(low_str..high_str))?;
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
        let iter = self.entries_db.iter(reader)?;
        let entries = iter
            .filter_map(|e| match e {
                Ok((_, e)) => Some(e),
                _ => None,
            })
            .collect();
        Ok(entries)
    }

    fn append(&self, writer: &mut heed::RwTxn, entries: &[Entry]) -> Result<()> {
        let mut last_index = self.last_index(writer)?;
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

    fn save_compacted_entries(&self, entries: &[Entry]) -> Result<()> {
        let compacted_log_dir_path = Path::new(&self.config.compacted_log_dir);
        let compacted_log_dir_path = compacted_log_dir_path.to_str().unwrap();
        let dest_path = format!(
            "{}/node-{}/compacted.json",
            compacted_log_dir_path, self.config.raft_config.id
        );

        match fs::metadata(&dest_path) {
            Ok(metadata) if metadata.len() > self.config.compacted_log_size_threshold => {
                println!(
                    "Compacted log size is over threshold. Removing all previous compacted logs."
                );
                fs::remove_file(&dest_path)?;
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => {
                return Err(e.into());
            }
            _ => {}
        }

        append_to_json_file(Path::new(&dest_path), entries)?;
        Ok(())
    }
}
