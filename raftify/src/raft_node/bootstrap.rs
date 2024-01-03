use bincode::serialize;
use prost::Message as PMessage;
use raft::{
    eraftpb::{ConfChange, ConfChangeType, Entry, EntryType},
    raw_node::RawNode,
};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::error::Result;
use crate::storage::heed::LogStore;
use crate::Peers;

/// Commit the configuration change to add all follower nodes to the cluster.
pub async fn bootstrap_peers<T: LogStore>(
    peers: Arc<Mutex<Peers>>,
    raw_node: &mut RawNode<T>,
) -> Result<()> {
    let initial_peers = peers.lock().await;

    let mut entries = vec![];
    let last_index = LogStore::last_index(raw_node.store()).unwrap();

    for (i, peer) in initial_peers.iter().enumerate() {
        let node_id = &peer.0;
        let node_addr = initial_peers.get(node_id).unwrap().addr;
        // Skip leader
        if *node_id == 1 {
            continue;
        }
        let mut conf_change = ConfChange::default();
        conf_change.set_node_id(*node_id);
        conf_change.set_change_type(ConfChangeType::AddNode);
        conf_change.set_context(serialize(&vec![node_addr]).unwrap());

        let conf_state = raw_node.apply_conf_change(&conf_change)?;
        raw_node.mut_store().set_conf_state(&conf_state)?;

        let mut entry = Entry::default();
        entry.set_entry_type(EntryType::EntryConfChange);
        entry.set_term(1);
        entry.set_index(last_index + i as u64);
        entry.set_data(conf_change.encode_to_vec());
        entry.set_context(vec![]);
        entries.push(entry);
    }

    let unstable = &mut raw_node.raft.raft_log.unstable;
    unstable.entries = entries.clone();
    let commit_index = last_index + entries.len() as u64;
    unstable.stable_entries(commit_index, 1);

    raw_node.mut_store().append(&entries)?;

    let last_applied = raw_node.raft.raft_log.applied;
    let store = raw_node.mut_store();
    store.compact(last_applied)?;
    store.create_snapshot(vec![], commit_index, 1)?;

    raw_node.raft.raft_log.applied = commit_index;
    raw_node.raft.raft_log.committed = commit_index;
    raw_node.raft.raft_log.persisted = commit_index;

    let leader_id = raw_node.raft.leader_id;
    let leader_pr = raw_node.raft.mut_prs().get_mut(leader_id).unwrap();

    leader_pr.matched = commit_index;
    leader_pr.committed_index = commit_index;
    leader_pr.next_idx = commit_index + 1;

    Ok(())
}
