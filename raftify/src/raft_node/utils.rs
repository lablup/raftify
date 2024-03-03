use serde_json::{json, Value};
use std::collections::HashMap;

use crate::{
    raft::{formatter::format_snapshot, RawNode},
    Result, StableStorage,
};

static EXPECTED_FORMAT_NOT_EXIST: &str = "Expected format not exist!";

pub fn format_debugging_info(hashmap: &HashMap<String, Value>) -> String {
    let node_id = hashmap
        .get("node_id")
        .and_then(|v| v.as_u64())
        .expect(EXPECTED_FORMAT_NOT_EXIST);

    let leader_id = hashmap
        .get("leader_id")
        .and_then(|v| v.as_u64())
        .expect(EXPECTED_FORMAT_NOT_EXIST);

    let term = hashmap
        .get("term")
        .and_then(|v| v.as_u64())
        .expect(EXPECTED_FORMAT_NOT_EXIST);

    let outline = format!(
        "========= Outline =========\n\
        node_id: {}\n\
        leader_id: {}\n\
        term: {}\n",
        node_id, leader_id, term
    );

    let storage = hashmap
        .get("storage")
        .and_then(|v| v.as_object())
        .cloned()
        .expect(EXPECTED_FORMAT_NOT_EXIST);

    let hard_state = storage
        .get("hard_state")
        .and_then(|v| v.as_object())
        .cloned()
        .expect(EXPECTED_FORMAT_NOT_EXIST);

    let conf_state = storage
        .get("conf_state")
        .and_then(|v| v.as_object())
        .cloned()
        .expect(EXPECTED_FORMAT_NOT_EXIST);

    let last_index = storage
        .get("last_index")
        .and_then(|v| v.as_number())
        .cloned()
        .expect(EXPECTED_FORMAT_NOT_EXIST);

    let snapshot = storage
        .get("snapshot")
        .and_then(|v| v.as_str())
        .expect(EXPECTED_FORMAT_NOT_EXIST);

    let hard_state_formatted = format!("{:?}", hard_state);
    let conf_state_formatted = format!("{:?}", conf_state);
    let snapshot_formatted = format!("{:?}", snapshot);

    let persistence_info = format!(
        "========= Persistence Info =========\n\
        hard_state: {}\n\
        conf_state: {}\n\
        last_index: {}\n\
        snapshot: {}\n",
        hard_state_formatted, conf_state_formatted, last_index, snapshot_formatted,
    );

    let progress = hashmap
        .get("progress")
        .and_then(|v| v.as_object().cloned())
        .expect(EXPECTED_FORMAT_NOT_EXIST);

    let progress_formatted = format!("{:?}", progress);
    let progress_info = format!(
        "========= Progress Info =========\n\
        {}\n",
        progress_formatted,
    );

    let raft_log = hashmap
        .get("raft_log")
        .and_then(|v| v.as_object().cloned())
        .expect(EXPECTED_FORMAT_NOT_EXIST);

    let raft_log_formatted = format!("{:?}", raft_log);
    let raft_log_info = format!(
        "========= RaftLog Info =========\n\
        {}\n",
        raft_log_formatted,
    );

    let result = format!("{outline}\n{persistence_info}\n{progress_info}\n{raft_log_info}\n");
    result
}

pub fn inspect_raftnode<T: StableStorage>(raw_node: &RawNode<T>) -> Result<String> {
    let id = raw_node.raft.id;
    let leader_id = raw_node.raft.leader_id;

    let prs = if id == leader_id {
        raw_node
            .raft
            .prs()
            .iter()
            .map(|(node_id, pr)| {
                (
                    node_id,
                    json!({
                        "matched": pr.matched,
                        "next_idx": pr.next_idx,
                        "paused": pr.paused,
                        "pending_snapshot": pr.pending_request_snapshot,
                        "pending_request_snapshot": pr.pending_request_snapshot,
                        "recent_active": pr.recent_active,
                        "commit_group_id": pr.commit_group_id,
                        "committed_index": pr.committed_index,
                        "ins": format!("{:?}", pr.ins),
                        "state": format!("{}", pr.state),
                    }),
                )
            })
            .collect::<HashMap<_, _>>()
    } else {
        HashMap::new()
    };

    let store = raw_node.store();

    let hard_state = store.hard_state()?;
    let conf_state = store.conf_state()?;
    let snapshot = store.snapshot(0, 0)?;
    let last_index = raw_node.raft.raft_log.last_index();

    let last_applied = raw_node.raft.raft_log.applied;
    let last_committed = raw_node.raft.raft_log.committed;
    let last_persisted = raw_node.raft.raft_log.persisted;

    let result = json!({
        "node_id": id,
        "leader_id": leader_id,
        "term": raw_node.raft.term,
        "storage": {
            "hard_state": {
                "term": hard_state.term,
                "vote": hard_state.vote,
                "commit": hard_state.commit,
            },
            "conf_state": {
                "voters": conf_state.voters,
                "learners": conf_state.learners,
                "voters_outgoing": conf_state.voters_outgoing,
                "learners_next": conf_state.learners_next,
            },
            "snapshot": format_snapshot(&snapshot),
            "last_index": last_index,
        },
        "progress": prs,
        "raft_log": {
            "committed": last_committed,
            "applied": last_applied,
            "persisted": last_persisted,
        },
    });

    Ok(result.to_string())
}
