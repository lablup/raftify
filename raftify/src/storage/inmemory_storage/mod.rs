use crate::{
    error::Result,
    raft::{
        self,
        eraftpb::{ConfState, Entry, HardState, Snapshot},
        storage::{MemStorage as MemStorageCore, Storage},
        INVALID_INDEX,
    },
    StableStorage,
};

#[derive(Clone)]
pub struct MemStorage {
    core: MemStorageCore,
    snapshot: Snapshot,
}

impl MemStorage {
    pub fn create() -> Self {
        let core = MemStorageCore::default();
        let snapshot: Snapshot = Default::default();
        Self { core, snapshot }
    }
}

impl StableStorage for MemStorage {
    fn append(&mut self, entries: &[Entry]) -> Result<()> {
        let mut store = self.core.wl();
        store.append(entries)?;
        Ok(())
    }

    fn hard_state(&self) -> Result<HardState> {
        let store = self.core.rl();
        Ok(store.hard_state().clone())
    }

    fn set_hard_state(&mut self, hard_state: &HardState) -> Result<()> {
        let mut store = self.core.wl();
        store.set_hardstate(hard_state.clone());
        Ok(())
    }

    fn set_hard_state_commit(&mut self, commit: u64) -> Result<()> {
        let mut store = self.core.wl();
        let mut hard_state = store.hard_state().clone();
        hard_state.set_commit(commit);
        store.set_hardstate(hard_state);
        Ok(())
    }

    fn conf_state(&self) -> Result<ConfState> {
        todo!()
    }

    fn set_conf_state(&mut self, conf_state: &ConfState) -> Result<()> {
        let mut store = self.core.wl();
        store.set_conf_state(conf_state.clone());
        Ok(())
    }

    fn create_snapshot(&mut self, data: Vec<u8>, index: u64, term: u64) -> Result<()> {
        let mut snapshot = self.core.snapshot(0, 0)?;
        snapshot.set_data(data);
        let mut metadata = snapshot.take_metadata();
        metadata.set_index(index);
        metadata.set_term(term);
        snapshot.set_metadata(metadata);
        self.snapshot = snapshot;
        Ok(())
    }

    fn apply_snapshot(&mut self, snapshot: Snapshot) -> Result<()> {
        let mut store = self.core.wl();

        // Pass apply snapshot if the snapshot is empty
        if snapshot.get_metadata().get_index() == INVALID_INDEX {
            // Update conf states.
            store.set_conf_state(snapshot.get_metadata().clone().take_conf_state());
            return Ok(());
        }

        store.apply_snapshot(snapshot)?;
        Ok(())
    }

    fn compact(&mut self, index: u64) -> Result<()> {
        let mut store = self.core.wl();
        store.compact(index)?;
        Ok(())
    }

    fn all_entries(&self) -> raft::Result<Vec<Entry>> {
        todo!()
    }
}

impl Storage for MemStorage {
    fn initial_state(&self) -> raft::Result<raft::prelude::RaftState> {
        let raft_state = self.core.initial_state()?;
        Ok(raft_state)
    }

    fn entries(
        &self,
        low: u64,
        high: u64,
        max_size: impl Into<Option<u64>>,
        context: raft::GetEntriesContext,
    ) -> raft::Result<Vec<Entry>> {
        let entries = self.core.entries(low, high, max_size, context)?;
        Ok(entries)
    }

    fn term(&self, idx: u64) -> raft::Result<u64> {
        self.core.term(idx)
    }

    fn first_index(&self) -> raft::Result<u64> {
        self.core.first_index()
    }

    fn last_index(&self) -> raft::Result<u64> {
        self.core.last_index()
    }

    fn snapshot(&self, _request_index: u64, _to: u64) -> raft::Result<Snapshot> {
        Ok(self.snapshot.clone())
    }
}
