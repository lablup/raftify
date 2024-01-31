use bincode::serialize;
use jopemachine_raft::Storage;
use prost::Message as PMessage;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::{
    error::Result,
    raft::{
        eraftpb::{ConfChange, ConfChangeType, Entry, EntryType},
        raw_node::RawNode,
    },
    storage::heed::LogStore,
    Peers,
};

/// Commit the configuration change to add all follower nodes to the cluster.
#[deprecated]
#[allow(dead_code)]
pub async fn bootstrap_peers<T: LogStore + Storage>(
    peers: Arc<Mutex<Peers>>,
    raw_node: &mut RawNode<T>,
) -> Result<()> {
    let mut initial_peers = peers.lock().await.clone();
    // Skip self (the leader node).
    initial_peers.remove(&raw_node.raft.id);

    let storage = raw_node.store();
    let last_index = LogStore::last_index(storage)?;
    let last_term = storage.term(last_index)?;

    let mut entries = vec![];
    for (i, peer) in initial_peers.iter().enumerate() {
        let node_id = &peer.0;
        let node_addr = initial_peers.get(node_id).unwrap().addr;

        let mut conf_change = ConfChange::default();
        conf_change.set_node_id(*node_id);
        conf_change.set_change_type(ConfChangeType::AddNode);
        conf_change.set_context(serialize(&vec![node_addr])?);

        let conf_state = raw_node.apply_conf_change(&conf_change)?;
        raw_node.mut_store().set_conf_state(&conf_state)?;

        let mut entry = Entry::default();
        entry.set_entry_type(EntryType::EntryConfChange);
        entry.set_index(last_index + 1 + i as u64);
        entry.set_term(last_term);
        entry.set_data(conf_change.encode_to_vec());
        entry.set_context(vec![]);
        entries.push(entry);
    }

    let commit_index = last_index + entries.len() as u64;

    {
        let unstable = &mut raw_node.raft.raft_log.unstable;
        unstable.entries = entries.clone();
        unstable.stable_entries(commit_index, last_term);

        let store = raw_node.mut_store();
        store.append(&entries)?;
    }

    {
        let last_applied = raw_node.raft.raft_log.applied;
        let store = raw_node.mut_store();
        let snapshot = LogStore::snapshot(store, 0, commit_index)?;

        store.compact(last_applied)?;
        store.create_snapshot(snapshot.get_data().to_vec(), last_applied, last_term)?;
    }

    {
        let raft_log = &mut raw_node.raft.raft_log;
        raft_log.applied = commit_index;
        raft_log.committed = commit_index;
        raft_log.persisted = commit_index;
    }

    let leader_id = raw_node.raft.leader_id;
    let leader_pr = raw_node.raft.mut_prs().get_mut(leader_id).unwrap();

    leader_pr.matched = commit_index;
    leader_pr.committed_index = commit_index;
    leader_pr.next_idx = commit_index + 1;

    Ok(())
}
