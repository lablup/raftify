mod codec;
mod constant;

use crate::raft::eraftpb::Entry;
use crate::raft::logger::Logger;
use crate::raft::prelude::{ConfState, HardState, Snapshot};
use crate::raft::{GetEntriesContext, RaftState, Storage};
use crate::{Result, StableStorage};
use codec::format_entry_key_string;
use constant::{
    CONF_STATE_KEY, HARD_STATE_KEY, LAST_INDEX_KEY, LOG_ENTRY_CF_KEY, METADATA_CF_KEY, SNAPSHOT_KEY,
};
use parking_lot::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use prost::Message;
use raft::util::limit_size;
use rocksdb::{ColumnFamilyDescriptor, Options, DB as RocksDB};
use std::cmp::max;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use super::StorageType;

#[derive(Clone)]
pub struct RocksDBStorage(Arc<RwLock<RocksDBStorageCore>>);

pub struct RocksDBStorageCore {
    db: RocksDB,
    logger: Arc<dyn Logger>,
}

impl RocksDBStorage {
    pub fn create(log_dir_path: &str, logger: Arc<dyn Logger>) -> Result<Self> {
        logger.trace("Used RocksDBStorage");
        Ok(Self(Arc::new(RwLock::new(RocksDBStorageCore::create(
            Path::new(log_dir_path).to_path_buf(),
            logger,
        )?))))
    }

    pub fn open_readonly(log_dir_path: &str, logger: Arc<dyn Logger>) -> Result<Self> {
        Ok(Self(Arc::new(RwLock::new(
            RocksDBStorageCore::open_readonly(Path::new(log_dir_path).to_path_buf(), logger)?,
        ))))
    }

    fn wl(&mut self) -> RwLockWriteGuard<RocksDBStorageCore> {
        self.0.write()
    }

    fn rl(&self) -> RwLockReadGuard<RocksDBStorageCore> {
        self.0.read()
    }
}

impl StableStorage for RocksDBStorage {
    const STORAGE_TYPE: StorageType = StorageType::RocksDB;

    fn append(&mut self, entries: &[Entry]) -> Result<()> {
        let mut store = self.wl();
        store.append(entries)
    }

    fn hard_state(&self) -> Result<HardState> {
        let store = self.rl();
        let hard_state = store.hard_state()?;
        Ok(hard_state)
    }

    fn set_hard_state(&mut self, hard_state: &HardState) -> Result<()> {
        let mut store = self.wl();
        store.set_hard_state(hard_state)?;
        Ok(())
    }

    fn set_hard_state_commit(&mut self, commit: u64) -> Result<()> {
        let mut store = self.wl();
        let mut hard_state = store.hard_state()?;
        hard_state.set_commit(commit);
        store.set_hard_state(&hard_state)?;
        Ok(())
    }

    fn conf_state(&self) -> Result<ConfState> {
        let store = self.rl();
        let conf_state = store.conf_state()?;
        Ok(conf_state)
    }

    fn set_conf_state(&mut self, conf_state: &ConfState) -> Result<()> {
        let mut store = self.wl();
        store.set_conf_state(conf_state)?;
        Ok(())
    }

    fn create_snapshot(&mut self, data: Vec<u8>, index: u64, term: u64) -> Result<()> {
        let mut store = self.wl();
        store.create_snapshot(data, index, term)?;
        Ok(())
    }

    fn apply_snapshot(&mut self, snapshot: Snapshot) -> Result<()> {
        let mut store = self.wl();
        store.apply_snapshot(snapshot)?;
        Ok(())
    }

    fn compact(&mut self, index: u64) -> Result<()> {
        let mut store = self.wl();
        store.compact(index)?;
        Ok(())
    }

    fn all_entries(&self) -> raft::Result<Vec<Entry>> {
        let store = self.rl();
        let entries = store.all_entries()?;
        Ok(entries)
    }
}

impl Storage for RocksDBStorage {
    fn initial_state(&self) -> crate::raft::Result<RaftState> {
        let store = self.rl();
        let state = store.initial_state()?;
        Ok(state)
    }

    fn entries(
        &self,
        low: u64,
        high: u64,
        max_size: Option<u64>,
        context: GetEntriesContext,
    ) -> crate::raft::Result<Vec<Entry>> {
        let store = self.rl();
        let entries = store.entries(low, high, max_size, context)?;
        Ok(entries)
    }

    fn term(&self, idx: u64) -> crate::raft::Result<u64> {
        let store = self.rl();
        let term = store.term(idx)?;
        Ok(term)
    }

    fn first_index(&self) -> crate::raft::Result<u64> {
        let store = self.rl();
        let index = store.first_index()?;
        Ok(index)
    }

    fn last_index(&self) -> crate::raft::Result<u64> {
        let store = self.rl();
        let index = store.last_index()?;
        Ok(index)
    }

    fn snapshot(&self, _request_index: u64, _to: u64) -> crate::raft::Result<Snapshot> {
        let store = self.rl();
        let snapshot = store.snapshot(_request_index, _to)?;
        Ok(snapshot)
    }
}

impl RocksDBStorageCore {
    pub fn create(path: PathBuf, logger: Arc<dyn Logger>) -> Result<Self> {
        let mut db_opts = Options::default();
        db_opts.create_if_missing(true);
        db_opts.create_missing_column_families(true);

        let cf_opts = Options::default();
        let cf_descriptors = vec![
            ColumnFamilyDescriptor::new(LOG_ENTRY_CF_KEY, cf_opts.clone()),
            ColumnFamilyDescriptor::new(METADATA_CF_KEY, cf_opts.clone()),
        ];

        let db = RocksDB::open_cf_descriptors(&db_opts, path, cf_descriptors).unwrap();
        Ok(RocksDBStorageCore { db, logger })
    }

    pub fn open_readonly(path: PathBuf, logger: Arc<dyn Logger>) -> Result<Self> {
        let mut db_opts = Options::default();
        db_opts.create_if_missing(true);
        db_opts.create_missing_column_families(true);

        let cf_opts = Options::default();
        let cf_descriptors = vec![
            ColumnFamilyDescriptor::new(LOG_ENTRY_CF_KEY, cf_opts.clone()),
            ColumnFamilyDescriptor::new(METADATA_CF_KEY, cf_opts.clone()),
        ];

        let db =
            RocksDB::open_cf_descriptors_read_only(&db_opts, path, cf_descriptors, false).unwrap();
        Ok(RocksDBStorageCore { db, logger })
    }

    #[allow(dead_code)]
    fn replace_entries(&self, entries: &[Entry]) -> crate::raft::Result<()> {
        let cf_handle = self.db.cf_handle(LOG_ENTRY_CF_KEY).unwrap();

        let mut last_index = self.last_index()?;
        let start = format_entry_key_string(0.to_string().as_str());
        let end = format_entry_key_string(last_index.to_string().as_str());

        self.db.delete_range_cf(cf_handle, start, end).unwrap();

        for entry in entries {
            last_index = std::cmp::max(entry.index, last_index);
            let index = format_entry_key_string(entry.index.to_string().as_str());
            self.db
                .put_cf(cf_handle, index, entry.encode_to_vec())
                .unwrap();
        }
        self.set_last_index(last_index).unwrap();
        Ok(())
    }

    fn append(&mut self, entries: &[Entry]) -> Result<()> {
        let cf_handle = self.db.cf_handle(LOG_ENTRY_CF_KEY).unwrap();

        if entries.is_empty() {
            return Ok(());
        }

        let mut last_index = self.last_index()?;

        if last_index + 1 < entries[0].index {
            self.logger.fatal(&format!(
                "raft logs should be continuous, last index: {}, new appended: {}",
                last_index, entries[0].index,
            ));
        }

        for entry in entries {
            last_index = std::cmp::max(entry.index, last_index);
            let index = format_entry_key_string(entry.index.to_string().as_str());
            self.db
                .put_cf(cf_handle, index, entry.encode_to_vec())
                .unwrap();
        }
        self.set_last_index(last_index)?;
        Ok(())
    }

    fn hard_state(&self) -> Result<HardState> {
        let cf_handle = self.db.cf_handle(METADATA_CF_KEY).unwrap();
        let result = self.db.get_cf(cf_handle, HARD_STATE_KEY).unwrap();
        match result {
            Some(data) => Ok(HardState::decode(&*data)?),
            None => Ok(HardState::default()),
        }
    }

    fn set_hard_state(&mut self, hard_state: &HardState) -> Result<()> {
        let cf_handle = self.db.cf_handle(METADATA_CF_KEY).unwrap();
        self.db
            .put_cf(cf_handle, HARD_STATE_KEY, hard_state.encode_to_vec())
            .unwrap();
        Ok(())
    }

    #[allow(dead_code)]
    fn set_hard_state_commit(&mut self, commit: u64) -> Result<()> {
        let mut hard_state = self.hard_state()?;
        hard_state.set_commit(commit);
        self.set_hard_state(&hard_state)
    }

    fn conf_state(&self) -> Result<ConfState> {
        let cf_handle = self.db.cf_handle(METADATA_CF_KEY).unwrap();
        let result = self.db.get_cf(cf_handle, CONF_STATE_KEY).unwrap();
        match result {
            Some(data) => Ok(ConfState::decode(&*data)?),
            None => Ok(ConfState::default()),
        }
    }

    fn set_conf_state(&mut self, conf_state: &ConfState) -> Result<()> {
        let cf_handle = self.db.cf_handle(METADATA_CF_KEY).unwrap();
        self.db
            .put_cf(cf_handle, CONF_STATE_KEY, conf_state.encode_to_vec())
            .unwrap();
        Ok(())
    }

    fn set_last_index(&self, index: u64) -> Result<()> {
        let cf_handle = self.db.cf_handle(METADATA_CF_KEY).unwrap();
        self.db
            .put_cf(cf_handle, LAST_INDEX_KEY, index.to_string().as_bytes())
            .unwrap();
        Ok(())
    }

    fn set_snapshot(&self, snapshot: &Snapshot) -> Result<()> {
        let cf_handle = self.db.cf_handle(METADATA_CF_KEY).unwrap();
        self.db
            .put_cf(cf_handle, SNAPSHOT_KEY, snapshot.encode_to_vec())
            .unwrap();
        Ok(())
    }

    fn create_snapshot(&mut self, data: Vec<u8>, index: u64, term: u64) -> Result<()> {
        let conf_state = self.conf_state().unwrap();
        let mut snapshot = Snapshot::default();
        snapshot.set_data(data);

        let meta = snapshot.mut_metadata();
        meta.set_conf_state(conf_state);
        meta.index = index;
        meta.term = term;

        self.set_snapshot(&snapshot)?;
        Ok(())
    }

    fn apply_snapshot(&mut self, snapshot: Snapshot) -> Result<()> {
        let metadata = snapshot.get_metadata();
        let conf_state = metadata.get_conf_state();

        let mut hard_state = self.hard_state()?;
        hard_state.set_term(max(hard_state.term, metadata.term));
        hard_state.set_commit(metadata.index);

        self.set_hard_state(&hard_state)?;
        self.set_conf_state(conf_state)?;
        self.set_last_index(metadata.index)?;
        self.set_snapshot(&snapshot)?;
        Ok(())
    }

    fn compact(&mut self, index: u64) -> Result<()> {
        let cf_handle = self.db.cf_handle(LOG_ENTRY_CF_KEY).unwrap();
        let start = format_entry_key_string(0.to_string().as_str());
        let end = format_entry_key_string((index).to_string().as_str());
        self.db.delete_range_cf(cf_handle, start, end).unwrap();
        Ok(())
    }

    fn all_entries(&self) -> crate::raft::Result<Vec<Entry>> {
        let mut entries = Vec::new();
        let cf_handle = self.db.cf_handle(LOG_ENTRY_CF_KEY).unwrap();

        let iter = self.db.iterator_cf(cf_handle, rocksdb::IteratorMode::Start);

        for item in iter {
            match item {
                Ok((_key, value)) => match Entry::decode(&*value) {
                    Ok(entry) => entries.push(entry),
                    Err(e) => {
                        return Err(crate::raft::Error::Store(crate::raft::StorageError::Other(
                            Box::new(e),
                        )))
                    }
                },
                Err(e) => {
                    return Err(crate::raft::Error::Store(crate::raft::StorageError::Other(
                        Box::new(e),
                    )))
                }
            }
        }

        Ok(entries)
    }

    fn initial_state(&self) -> crate::raft::Result<RaftState> {
        let hard_state = self.hard_state().unwrap();
        let conf_state = self.conf_state().unwrap();
        Ok(RaftState {
            hard_state,
            conf_state,
        })
    }

    fn entries(
        &self,
        low: u64,
        high: u64,
        max_size: Option<u64>,
        _context: GetEntriesContext,
    ) -> crate::raft::Result<Vec<Entry>> {
        let cf_handle = self.db.cf_handle(LOG_ENTRY_CF_KEY).unwrap();
        let mut entries = Vec::new();

        if low
            < self
                .first_index()
                .map_err(|_| raft::Error::Store(raft::StorageError::Unavailable))?
        {
            return Err(raft::Error::Store(raft::StorageError::Compacted));
        }

        for idx in low..high {
            let index = format_entry_key_string(idx.to_string().as_str());
            match self.db.get_cf(cf_handle, index).unwrap() {
                Some(value) => {
                    let entry = Entry::decode(&*value).unwrap();
                    entries.push(entry);
                }
                None => continue,
            }
        }

        limit_size(&mut entries, max_size);

        Ok(entries)
    }

    fn term(&self, idx: u64) -> crate::raft::Result<u64> {
        let cf_handle = self.db.cf_handle(LOG_ENTRY_CF_KEY).unwrap();
        let first_index = self.first_index()?;

        let snapshot = self.snapshot(0, 0)?;
        if snapshot.get_metadata().get_index() == idx {
            return Ok(snapshot.get_metadata().get_term());
        }

        let index = format_entry_key_string(idx.to_string().as_str());

        if let Some(value) = self.db.get_cf(cf_handle, index).unwrap() {
            match Entry::decode(&*value) {
                Ok(entry) => Ok(entry.term),
                Err(_e) => Err(crate::raft::Error::Store(
                    crate::raft::StorageError::Unavailable,
                )),
            }
        } else if idx < first_index {
            Err(crate::raft::Error::Store(
                crate::raft::StorageError::Compacted,
            ))
        } else {
            Err(crate::raft::Error::Store(
                crate::raft::StorageError::Unavailable,
            ))
        }
    }

    fn first_index(&self) -> crate::raft::Result<u64> {
        let cf_handle = self.db.cf_handle(LOG_ENTRY_CF_KEY).unwrap();
        let mut iter = self.db.iterator_cf(cf_handle, rocksdb::IteratorMode::Start);

        match iter.next() {
            Some(first) => {
                let (_, value) = first.unwrap();
                let entry = Entry::decode(&*value).unwrap();
                Ok(entry.index)
            }
            None => {
                let snapshot = self.snapshot(0, 0).unwrap();
                Ok(snapshot.get_metadata().get_index() + 1)
            }
        }
    }

    fn last_index(&self) -> crate::raft::Result<u64> {
        let cf_handle = self.db.cf_handle(METADATA_CF_KEY).unwrap();
        let last_index = self
            .db
            .get_cf(cf_handle, LAST_INDEX_KEY)
            .unwrap()
            .unwrap_or("0".as_bytes().to_vec());
        let last_index = String::from_utf8(last_index).expect("Found invalid UTF-8");
        Ok(last_index.parse().unwrap())
    }

    fn snapshot(&self, _request_index: u64, _to: u64) -> crate::raft::Result<Snapshot> {
        let cf_handle = self.db.cf_handle(METADATA_CF_KEY).unwrap();

        match self.db.get_cf(cf_handle, SNAPSHOT_KEY) {
            Ok(Some(value)) => {
                let snapshot = Snapshot::decode(&*value).unwrap();
                Ok(snapshot)
            }
            Ok(None) => Ok(Snapshot::default()),
            Err(_) => Err(crate::raft::Error::Store(
                crate::raft::StorageError::SnapshotTemporarilyUnavailable,
            )),
        }
    }
}

#[cfg(test)]
mod test {
    use std::fs;
    use std::panic::{self, AssertUnwindSafe};
    use std::sync::Arc;

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

        Config {
            log_dir: test_dir_pth.to_owned(),
            save_compacted_logs: false,
            compacted_log_dir: test_dir_pth.to_owned(),
            compacted_log_size_threshold: 1024 * 1024 * 1024,
            raft_config,
            ..Default::default()
        }
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
            // (
            //     vec![new_entry(2, 3), new_entry(3, 3), new_entry(4, 5)],
            //     None,
            // ),
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
