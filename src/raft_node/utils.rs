use raft::{derializer::format_snapshot, RawNode};

use crate::{LogStore, Result};

pub fn inspect_raftnode<T: LogStore>(raw_node: &RawNode<T>) -> Result<String> {
    let id = raw_node.raft.id;
    let prs = raw_node.raft.prs().get(id).unwrap();
    let store = raw_node.store();

    let leader_id = raw_node.raft.leader_id;
    let hard_state = store.hard_state()?;
    let conf_state = store.conf_state()?;
    let snapshot = LogStore::snapshot(store, 0, 0)?;
    let last_index = raw_node.raft.raft_log.last_index();

    let last_applied = raw_node.raft.raft_log.applied;
    let last_committed = raw_node.raft.raft_log.committed;
    let last_persisted = raw_node.raft.raft_log.persisted;

    let outline = format!(
        "========= Outline =========\n\
        node_id: {}\n\
        leader_id: {}\n",
        id, leader_id
    );

    let persistence_info = format!(
        "========= Persistence Info =========\n\
        hard_state: {:?}\n\
        conf_state: {:?}\n\
        last_index: {}\n\
        snapshot: {:?}\n",
        hard_state,
        conf_state,
        last_index,
        format_snapshot(&snapshot),
    );

    let prs_info = format!(
        "========= Progresses =========\n\
        {:?}
        ",
        prs
    );

    let raftlog_metadata = format!(
        "========= RaftLog Metadata =========\n\
        last_applied: {}\n\
        last_committed: {}\n\
        last_persisted: {}\n",
        last_applied, last_committed, last_persisted
    );

    let result = format!("{outline:}\n{persistence_info:}\n{prs_info}\n{raftlog_metadata:}",);

    Ok(result)
}
