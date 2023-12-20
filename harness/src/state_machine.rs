use async_trait::async_trait;
use bincode::{deserialize, serialize};
use raftify::{AbstractLogEntry, AbstractStateMachine, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum LogEntry {
    Insert { key: u64, value: String },
}

impl AbstractLogEntry for LogEntry {
    fn encode(&self) -> Result<Vec<u8>> {
        serialize(self).map_err(|e| e.into())
    }

    fn decode(bytes: &[u8]) -> Result<LogEntry> {
        let log_entry: LogEntry = deserialize(bytes)?;
        Ok(log_entry)
    }
}

#[derive(Clone, Debug)]
pub struct HashStore(pub Arc<RwLock<HashMap<u64, String>>>);

impl HashStore {
    pub fn new() -> Self {
        Self(Arc::new(RwLock::new(HashMap::new())))
    }

    pub fn get(&self, id: u64) -> Option<String> {
        self.0.read().unwrap().get(&id).cloned()
    }
}

#[async_trait]
impl AbstractStateMachine<LogEntry> for HashStore {
    async fn apply(&mut self, log_entry: LogEntry) -> Result<LogEntry> {
        match log_entry {
            LogEntry::Insert { ref key, ref value } => {
                let mut db = self.0.write().unwrap();
                log::info!("Inserted: ({}, {})", key, value);
                db.insert(*key, value.clone());
            }
        };
        Ok(log_entry)
    }

    async fn snapshot(&self) -> Result<HashStore> {
        let snapshot = self.0.read().unwrap().clone();
        Ok(HashStore(Arc::new(RwLock::new(snapshot))))
    }

    async fn restore(&mut self, snapshot: HashStore) -> Result<()> {
        let new: HashMap<u64, String> = snapshot.0.read().unwrap().clone();
        let mut db = self.0.write().unwrap();
        let _ = std::mem::replace(&mut *db, new);
        Ok(())
    }

    fn encode(&self) -> Result<Vec<u8>> {
        serialize(&self.0.read().unwrap().clone()).map_err(|e| e.into())
    }

    fn decode(bytes: &[u8]) -> Result<Self> {
        let db: HashMap<u64, String> = deserialize(bytes)?;
        Ok(Self(Arc::new(RwLock::new(db))))
    }
}
