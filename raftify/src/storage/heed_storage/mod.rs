mod codec;
mod constant;

use self::codec::{format_entry_key_string, HeedEntry, HeedEntryKeyString};
use super::{utils::append_compacted_logs, StableStorage, StorageType};
use crate::{
    config::Config,
    error::Result,
    raft::{self, prelude::*, GetEntriesContext},
};
use bincode::{deserialize, serialize};
use constant::{CONF_STATE_KEY, HARD_STATE_KEY, LAST_INDEX_KEY, SNAPSHOT_KEY};
use heed::{
    types::{Bytes as HeedBytes, Str as HeedStr},
    Database, Env,
};
use parking_lot::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use prost::Message as PMessage;
use raft::{logger::Logger, util::limit_size};
use std::{
    cmp::max,
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};

#[derive(Clone)]
pub struct HeedStorage(Arc<RwLock<HeedStorageCore>>);

impl HeedStorage {
    pub fn create(log_dir_path: &str, config: &Config, logger: Arc<dyn Logger>) -> Result<Self> {
        logger.trace("Used HeedStorage");
        Ok(Self(Arc::new(RwLock::new(HeedStorageCore::create(
            Path::new(log_dir_path).to_path_buf(),
            config,
            logger,
        )?))))
    }

    fn wl(&mut self) -> RwLockWriteGuard<HeedStorageCore> {
        self.0.write()
    }

    fn rl(&self) -> RwLockReadGuard<HeedStorageCore> {
        self.0.read()
    }
}

impl StableStorage for HeedStorage {
    const STORAGE_TYPE: StorageType = StorageType::Heed;

    fn compact(&mut self, index: u64) -> Result<()> {
        let store = self.wl();
        let mut writer = store.env.write_txn()?;
        store.compact(&mut writer, index)?;
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

        // TODO: Investigate if this is necessary. It broke the static bootstrap.
        // let first_index = store.first_index(&writer)?;
        // if first_index > metadata.index {
        //     return Err(Error::RaftStorageError(
        //         raft::StorageError::SnapshotOutOfDate,
        //     ));
        // }

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
        max_size: Option<u64>,
        ctx: GetEntriesContext,
    ) -> raft::Result<Vec<Entry>> {
        let store = self.rl();
        let reader = store.env.read_txn().unwrap();

        if low
            < store
                .first_index(&reader)
                .map_err(|_| raft::Error::Store(raft::StorageError::Unavailable))?
        {
            return Err(raft::Error::Store(raft::StorageError::Compacted));
        }

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

impl HeedStorage {
    #[allow(dead_code)]
    pub fn replace_entries(&mut self, entries: &[Entry]) -> raft::Result<()> {
        let store = self.wl();
        let mut writer = store.env.write_txn().unwrap();
        store.replace_entries(&mut writer, entries).unwrap();
        writer.commit().unwrap();
        Ok(())
    }
}

pub struct HeedStorageCore {
    env: Env,
    entries_db: Database<HeedEntryKeyString, HeedEntry>,
    metadata_db: Database<HeedStr, HeedBytes>,
    config: Config,
    logger: Arc<dyn Logger>,
}

impl HeedStorageCore {
    pub fn create(log_dir_path: PathBuf, config: &Config, logger: Arc<dyn Logger>) -> Result<Self> {
        let env = unsafe {
            heed::EnvOpenOptions::new()
                .map_size(config.lmdb_map_size as usize)
                .max_dbs(3000)
                .open(log_dir_path)?
        };

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

        Ok(storage)
    }

    pub fn compact(&self, writer: &mut heed::RwTxn, index: u64) -> Result<()> {
        // TODO, check that compaction is legal
        //let last_index = self.last_index(&writer)?;
        // there should always be at least one entry in the log
        //assert!(last_index > index + 1);

        let index = format_entry_key_string(index.to_string().as_str());

        if self.config.save_compacted_logs {
            let iter = self.entries_db.range(writer, &(..index.clone()))?;

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
        match self.entries_db.first(reader)? {
            Some((index, _first_entry)) => Ok(index.parse::<u64>().unwrap()),
            None => {
                // TODO: Pass proper arguments after handling snapshot's `request_index` and `to`
                let snapshot = self.snapshot(reader, 0, 0)?;
                Ok(snapshot.get_metadata().get_index() + 1)
            }
        }
    }

    fn entry(&self, reader: &heed::RoTxn, index: u64) -> Result<Option<Entry>> {
        Ok(self.entries_db.get(reader, &index.to_string())?)
    }

    fn entries(
        &self,
        reader: &heed::RoTxn,
        low: u64,
        high: u64,
        max_size: Option<u64>,
        _ctx: GetEntriesContext,
    ) -> Result<Vec<Entry>> {
        self.logger
            .debug(format!("Entries [{low}, {high}) requested.", low = low, high = high).as_str());

        let low_str = format_entry_key_string(low.to_string().as_str());
        let high_str = format_entry_key_string(high.to_string().as_str());

        let iter = self.entries_db.range(reader, &(low_str..high_str))?;
        let max_size: Option<u64> = max_size;

        let mut entries = iter
            .filter_map(|e| match e {
                Ok((_, e)) => Some(e),
                _ => None,
            })
            .collect();

        limit_size(&mut entries, max_size);
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
        if entries.is_empty() {
            return Ok(());
        }

        let first_index = self.first_index(writer)?;

        if first_index > entries[0].index {
            self.logger.fatal(&format!(
                "overwrite compacted raft logs, compacted: {}, append: {}",
                first_index - 1,
                entries[0].index,
            ));
        }

        let mut last_index = self.last_index(writer)?;

        if last_index + 1 < entries[0].index {
            self.logger.fatal(&format!(
                "raft logs should be continuous, last index: {}, new appended: {}",
                last_index, entries[0].index,
            ));
        }

        for entry in entries {
            let index = entry.index;
            last_index = std::cmp::max(index, last_index);
            // TODO: Handle MDBFullError.
            self.entries_db.put(writer, &index.to_string(), entry)?;
        }
        self.set_last_index(writer, last_index)?;
        Ok(())
    }

    fn save_compacted_entries(&self, entries: &[Entry]) -> Result<()> {
        let compacted_log_dir_path = Path::new(&self.config.compacted_log_dir);
        let compacted_log_dir_path = compacted_log_dir_path.to_str().unwrap();
        let dest_path = format!("{}/compacted_logs.json", compacted_log_dir_path);

        match fs::metadata(&dest_path) {
            Ok(metadata) if metadata.len() > self.config.compacted_log_size_threshold => {
                self.logger.debug(
                    "Compacted log size is over threshold. Removing all previous compacted logs.",
                );
                fs::remove_file(&dest_path)?;
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e.into()),
            _ => {}
        }

        append_compacted_logs(Path::new(&dest_path), entries)?;
        Ok(())
    }

    // Test only
    #[allow(dead_code)]
    fn replace_entries(&self, writer: &mut heed::RwTxn, entries: &[Entry]) -> Result<()> {
        self.entries_db.clear(writer)?;
        let mut last_index = self.last_index(writer)?;

        for entry in entries {
            let index = entry.index;
            last_index = std::cmp::max(index, last_index);
            // TODO: Handle MDBFullError.
            self.entries_db.put(writer, &index.to_string(), entry)?;
        }
        self.set_last_index(writer, last_index)?;
        Ok(())
    }
}

// Ref: https://github.com/tikv/raft-rs/blob/master/src/storage.rs
#[cfg(test)]
mod test {
    use std::fs;
    use std::panic::{self, AssertUnwindSafe};
    use std::sync::Arc;

    use crate::config::config_builder::ConfigBuilder;
    use crate::raft::{
        default_logger,
        eraftpb::{Entry, Snapshot},
        logger::Slogger,
        Config as RaftConfig, Error as RaftError, GetEntriesContext, Storage, StorageError,
    };
    use crate::{Config, HeedStorage, StableStorage};
    use prost::Message;

    fn new_entry(index: u64, term: u64) -> Entry {
        let mut e = Entry::default();
        e.term = term;
        e.index = index;
        e
    }

    fn size_of<T: Message>(m: &T) -> u32 {
        m.encoded_len() as u32
    }

    fn new_snapshot(index: u64, term: u64, voters: Vec<u64>) -> Snapshot {
        let mut s = Snapshot::default();
        s.mut_metadata().index = index;
        s.mut_metadata().term = term;
        s.mut_metadata().mut_conf_state().voters = voters;
        s
    }

    pub fn build_config(test_dir_pth: &str) -> Config {
        let raft_config = RaftConfig {
            election_tick: 10,
            heartbeat_tick: 3,
            ..Default::default()
        };

        ConfigBuilder::new()
            .log_dir(test_dir_pth.to_owned())
            .save_compacted_logs(false)
            .compacted_log_dir(test_dir_pth.to_owned())
            .compacted_log_size_threshold(1024 * 1024 * 1024)
            .raft_config(raft_config)
            .build()
    }

    pub fn build_logger() -> slog::Logger {
        default_logger()
    }

    fn setup() -> String {
        let tempdir = tempfile::tempdir()
            .expect("")
            .path()
            .to_str()
            .unwrap()
            .to_owned();
        fs::create_dir_all(&tempdir).expect("Failed to create test directory");

        tempdir
    }

    fn teardown(tempdir: String) {
        fs::remove_dir_all(tempdir).expect("Failed to delete test directory");
    }

    #[test]
    fn test_storage_term() {
        let tempdir = setup();

        let cfg: Config = build_config(&tempdir);
        let ents = vec![new_entry(3, 3), new_entry(4, 4), new_entry(5, 5)];
        let mut tests = vec![
            (2u64, Err(RaftError::Store(StorageError::Compacted))),
            (3u64, Ok(3)),
            (4u64, Ok(4)),
            (5u64, Ok(5)),
            (6u64, Err(RaftError::Store(StorageError::Unavailable))),
        ];

        for (i, (idx, wterm)) in tests.drain(..).enumerate() {
            let mut storage = HeedStorage::create(
                &tempdir,
                &cfg,
                Arc::new(Slogger {
                    slog: build_logger(),
                }),
            )
            .unwrap();

            storage.replace_entries(ents.as_slice()).unwrap();

            let t = storage.term(idx);
            if t != wterm {
                panic!("#{}: expect res {:?}, got {:?}", i, wterm, t);
            }
        }

        teardown(tempdir);
    }

    #[test]
    fn test_storage_entries() {
        let tempdir = setup();

        let cfg = build_config(&tempdir);

        let ents = vec![
            new_entry(3, 3),
            new_entry(4, 4),
            new_entry(5, 5),
            new_entry(6, 6),
        ];
        let max_u64 = u64::max_value();
        let mut tests = vec![
            (
                2,
                6,
                max_u64,
                Err(RaftError::Store(StorageError::Compacted)),
            ),
            (3, 4, max_u64, Ok(vec![new_entry(3, 3)])),
            (4, 5, max_u64, Ok(vec![new_entry(4, 4)])),
            (4, 6, max_u64, Ok(vec![new_entry(4, 4), new_entry(5, 5)])),
            (
                4,
                7,
                max_u64,
                Ok(vec![new_entry(4, 4), new_entry(5, 5), new_entry(6, 6)]),
            ),
            // even if maxsize is zero, the first entry should be returned
            (4, 7, 0, Ok(vec![new_entry(4, 4)])),
            // limit to 2
            (
                4,
                7,
                u64::from(size_of(&ents[1]) + size_of(&ents[2])),
                Ok(vec![new_entry(4, 4), new_entry(5, 5)]),
            ),
            (
                4,
                7,
                u64::from(size_of(&ents[1]) + size_of(&ents[2]) + size_of(&ents[3]) / 2),
                Ok(vec![new_entry(4, 4), new_entry(5, 5)]),
            ),
            (
                4,
                7,
                u64::from(size_of(&ents[1]) + size_of(&ents[2]) + size_of(&ents[3]) - 1),
                Ok(vec![new_entry(4, 4), new_entry(5, 5)]),
            ),
            // all
            (
                4,
                7,
                u64::from(size_of(&ents[1]) + size_of(&ents[2]) + size_of(&ents[3])),
                Ok(vec![new_entry(4, 4), new_entry(5, 5), new_entry(6, 6)]),
            ),
        ];
        for (i, (lo, hi, maxsize, wentries)) in tests.drain(..).enumerate() {
            let mut storage = HeedStorage::create(
                &tempdir,
                &cfg,
                Arc::new(Slogger {
                    slog: build_logger(),
                }),
            )
            .unwrap();

            storage.replace_entries(&ents).unwrap();

            let e = storage.entries(lo, hi, Some(maxsize), GetEntriesContext::empty(false));
            if e != wentries {
                panic!("#{}: expect entries {:?}, got {:?}", i, wentries, e);
            }
        }

        teardown(tempdir);
    }

    #[test]
    fn test_storage_last_index() {
        let tempdir = setup();

        let cfg = build_config(&tempdir);
        let logger = Arc::new(Slogger {
            slog: build_logger(),
        });
        let ents = vec![new_entry(3, 3), new_entry(4, 4), new_entry(5, 5)];
        let mut storage = HeedStorage::create(&tempdir, &cfg, logger).unwrap();
        storage.replace_entries(&ents).unwrap();

        let wresult = Ok(5);
        let result = storage.last_index();
        if result != wresult {
            panic!("want {:?}, got {:?}", wresult, result);
        }

        storage.append([new_entry(6, 5)].as_ref()).unwrap();
        let wresult = Ok(6);
        let result = storage.last_index();
        if result != wresult {
            panic!("want {:?}, got {:?}", wresult, result);
        }

        teardown(tempdir);
    }

    #[test]
    fn test_storage_first_index() {
        let tempdir = setup();

        let cfg = build_config(&tempdir);
        let logger = Arc::new(Slogger {
            slog: build_logger(),
        });
        let ents = vec![new_entry(3, 3), new_entry(4, 4), new_entry(5, 5)];
        let mut storage = HeedStorage::create(&tempdir, &cfg, logger).unwrap();
        storage.replace_entries(&ents).unwrap();

        assert_eq!(storage.first_index(), Ok(3));
        storage.compact(4).unwrap();
        assert_eq!(storage.first_index(), Ok(4));

        teardown(tempdir);
    }

    #[test]
    fn test_storage_compact() {
        let tempdir = setup();
        let cfg = build_config(&tempdir);
        let logger = Arc::new(Slogger {
            slog: build_logger(),
        });

        let ents = vec![new_entry(3, 3), new_entry(4, 4), new_entry(5, 5)];
        let mut tests = vec![(2, 3, 3, 3), (3, 3, 3, 3), (4, 4, 4, 2), (5, 5, 5, 1)];
        for (i, (idx, windex, wterm, wlen)) in tests.drain(..).enumerate() {
            let mut storage = HeedStorage::create(&tempdir, &cfg, logger.clone()).unwrap();
            storage.replace_entries(&ents).unwrap();

            storage.compact(idx).unwrap();
            let index = storage.first_index().unwrap();
            if index != windex {
                panic!("#{}: want {}, index {}", i, windex, index);
            }
            let term = if let Ok(v) =
                storage.entries(index, index + 1, Some(1), GetEntriesContext::empty(false))
            {
                v.first().map_or(0, |e| e.term)
            } else {
                0
            };
            if term != wterm {
                panic!("#{}: want {}, term {}", i, wterm, term);
            }
            let last = storage.last_index().unwrap();
            let len = storage
                .entries(index, last + 1, Some(100), GetEntriesContext::empty(false))
                .unwrap()
                .len();
            if len != wlen {
                panic!("#{}: want {}, term {}", i, wlen, len);
            }
        }

        teardown(tempdir);
    }

    // TODO: Support the below test case
    // #[test]
    // fn test_storage_create_snapshot() {
    //     let tempdir = setup();
    //     let cfg = build_config(&tempdir);
    //     let logger = Arc::new(Slogger {
    //         slog: build_logger(),
    //     });

    //     let ents = vec![new_entry(3, 3), new_entry(4, 4), new_entry(5, 5)];
    //     let nodes = vec![1, 2, 3];
    //     let mut conf_state = ConfState::default();
    //     conf_state.voters = nodes.clone();

    //     let unavailable = Err(RaftError::Store(
    //         StorageError::SnapshotTemporarilyUnavailable,
    //     ));
    //     let mut tests = vec![
    //         (4, Ok(new_snapshot(4, 4, nodes.clone())), 0),
    //         (5, Ok(new_snapshot(5, 5, nodes.clone())), 5),
    //         (5, Ok(new_snapshot(6, 5, nodes)), 6),
    //         (5, unavailable, 6),
    //     ];
    //     for (i, (idx, wresult, windex)) in tests.drain(..).enumerate() {
    //         let mut storage = HeedStorage::create(&tempdir, &cfg, logger.clone()).unwrap();
    //         storage.wl().entries = ents.clone();
    //         storage.wl().raft_state.hard_state.commit = idx;
    //         storage.wl().raft_state.hard_state.term = idx;
    //         storage.wl().raft_state.conf_state = conf_state.clone();

    //         // if wresult.is_err() {
    //         //     storage.wl().trigger_snap_unavailable();
    //         // }

    //         let result = storage.snapshot(windex, 0);
    //         if result != wresult {
    //             panic!("#{}: want {:?}, got {:?}", i, wresult, result);
    //         }
    //     }

    //     teardown(tempdir);
    // }

    #[test]
    fn test_storage_append() {
        let tempdir = setup();
        let cfg = build_config(&tempdir);
        let logger = Arc::new(Slogger {
            slog: build_logger(),
        });
        let ents = vec![new_entry(3, 3), new_entry(4, 4), new_entry(5, 5)];
        let mut tests = vec![
            (
                vec![new_entry(3, 3), new_entry(4, 4), new_entry(5, 5)],
                Some(vec![new_entry(3, 3), new_entry(4, 4), new_entry(5, 5)]),
            ),
            (
                vec![new_entry(3, 3), new_entry(4, 6), new_entry(5, 6)],
                Some(vec![new_entry(3, 3), new_entry(4, 6), new_entry(5, 6)]),
            ),
            (
                vec![
                    new_entry(3, 3),
                    new_entry(4, 4),
                    new_entry(5, 5),
                    new_entry(6, 5),
                ],
                Some(vec![
                    new_entry(3, 3),
                    new_entry(4, 4),
                    new_entry(5, 5),
                    new_entry(6, 5),
                ]),
            ),
            // TODO: Support the below test cases
            // overwrite compacted raft logs is not allowed
            (
                vec![new_entry(2, 3), new_entry(3, 3), new_entry(4, 5)],
                None,
            ),
            // truncate the existing entries and append
            // (
            //     vec![new_entry(4, 5)],
            //     Some(vec![new_entry(3, 3), new_entry(4, 5)]),
            // ),
            // direct append
            (
                vec![new_entry(6, 6)],
                Some(vec![
                    new_entry(3, 3),
                    new_entry(4, 4),
                    new_entry(5, 5),
                    new_entry(6, 6),
                ]),
            ),
        ];
        for (i, (entries, wentries)) in tests.drain(..).enumerate() {
            let mut storage = HeedStorage::create(&tempdir, &cfg, logger.clone()).unwrap();
            storage.replace_entries(&ents).unwrap();
            let res = panic::catch_unwind(AssertUnwindSafe(|| storage.append(&entries)));
            if let Some(wentries) = wentries {
                let _ = res.unwrap();
                let e = &storage.all_entries().unwrap();
                if *e != wentries {
                    panic!("#{}: want {:?}, entries {:?}", i, wentries, e);
                }
            } else {
                res.unwrap_err();
            }
        }

        teardown(tempdir);
    }

    #[test]
    fn test_storage_apply_snapshot() {
        let tempdir = setup();
        let cfg = build_config(&tempdir);
        let logger = Arc::new(Slogger {
            slog: build_logger(),
        });

        let nodes = vec![1, 2, 3];
        let mut storage = HeedStorage::create(&tempdir, &cfg, logger).unwrap();

        // Apply snapshot successfully
        let snap = new_snapshot(4, 4, nodes.clone());
        storage.apply_snapshot(snap).unwrap();

        // Apply snapshot fails due to StorageError::SnapshotOutOfDate
        // TODO: Support the below test case
        // let snap = new_snapshot(3, 3, nodes);
        // storage.apply_snapshot(snap).unwrap_err();

        teardown(tempdir);
    }
}
