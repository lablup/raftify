use pyo3::prelude::*;
use raftify::raft::Config;

use super::readonly_option::PyReadOnlyOption;

fn format_config<T: Into<Config>>(cfg: T) -> String {
    let cfg: Config = cfg.into();

    format!(
        "RaftConfig {{ \
            id: {:?}, \
            election_tick: {:?}, \
            heartbeat_tick: {:?}, \
            applied: {:?}, \
            max_size_per_msg: {:?}, \
            max_inflight_msgs: {:?}, \
            check_quorum: {:?}, \
            pre_vote: {:?}, \
            min_election_tick: {:?}, \
            max_election_tick: {:?}, \
            read_only_option: {:?}, \
            skip_bcast_commit: {:?}, \
            batch_append: {:?}, \
            priority: {:?}, \
            max_uncommitted_size: {:?}, \
            max_committed_size_per_ready: {:?} \
        }}",
        cfg.id,
        cfg.election_tick,
        cfg.heartbeat_tick,
        cfg.applied,
        cfg.max_size_per_msg,
        cfg.max_inflight_msgs,
        cfg.check_quorum,
        cfg.pre_vote,
        cfg.min_election_tick,
        cfg.max_election_tick,
        cfg.read_only_option,
        cfg.skip_bcast_commit,
        cfg.batch_append,
        cfg.priority,
        cfg.max_uncommitted_size,
        cfg.max_committed_size_per_ready,
    )
}

#[derive(Clone, Default)]
#[pyclass(name = "RaftConfig")]
pub struct PyRaftConfig {
    pub inner: Config,
}

#[pymethods]
impl PyRaftConfig {
    #![allow(clippy::too_many_arguments)]
    #[new]
    pub fn new(
        id: Option<u64>,
        election_tick: Option<usize>,
        heartbeat_tick: Option<usize>,
        applied: Option<u64>,
        max_size_per_msg: Option<u64>,
        max_inflight_msgs: Option<usize>,
        check_quorum: Option<bool>,
        pre_vote: Option<bool>,
        min_election_tick: Option<usize>,
        max_election_tick: Option<usize>,
        read_only_option: Option<&PyReadOnlyOption>,
        skip_bcast_commit: Option<bool>,
        batch_append: Option<bool>,
        priority: Option<i64>,
        max_uncommitted_size: Option<u64>,
        max_committed_size_per_ready: Option<u64>,
    ) -> Self {
        let mut config = Config::default();

        config.applied = applied.unwrap_or(config.applied);
        config.batch_append = batch_append.unwrap_or(config.batch_append);
        config.check_quorum = check_quorum.unwrap_or(config.check_quorum);
        config.election_tick = election_tick.unwrap_or(config.election_tick);
        config.heartbeat_tick = heartbeat_tick.unwrap_or(config.heartbeat_tick);
        config.id = id.unwrap_or(config.id);
        config.max_committed_size_per_ready =
            max_committed_size_per_ready.unwrap_or(config.max_committed_size_per_ready);
        config.max_inflight_msgs = max_inflight_msgs.unwrap_or(config.max_inflight_msgs);
        config.max_size_per_msg = max_size_per_msg.unwrap_or(config.max_size_per_msg);
        config.max_uncommitted_size = max_uncommitted_size.unwrap_or(config.max_uncommitted_size);
        config.min_election_tick = min_election_tick.unwrap_or(config.min_election_tick);
        config.max_election_tick = max_election_tick.unwrap_or(config.max_election_tick);
        config.pre_vote = pre_vote.unwrap_or(config.pre_vote);
        config.priority = priority.unwrap_or(config.priority);
        config.skip_bcast_commit = skip_bcast_commit.unwrap_or(config.skip_bcast_commit);
        config.read_only_option = read_only_option.map_or(config.read_only_option, |opt| opt.0);

        PyRaftConfig { inner: config }
    }

    pub fn __repr__(&self) -> String {
        format_config(self.inner.clone())
    }

    pub fn min_election_tick(&self) -> usize {
        self.inner.min_election_tick()
    }

    pub fn set_min_election_tick(&mut self, v: usize) {
        self.inner.min_election_tick = v;
    }

    pub fn max_election_tick(&self) -> usize {
        self.inner.max_election_tick()
    }

    pub fn set_max_election_tick(&mut self, v: usize) {
        self.inner.max_election_tick = v;
    }

    pub fn validate(&self) {
        self.inner.validate().unwrap();
    }

    pub fn get_id(&self) -> u64 {
        self.inner.id
    }

    pub fn set_id(&mut self, id: u64) {
        self.inner.id = id;
    }

    pub fn get_election_tick(&self) -> usize {
        self.inner.election_tick
    }

    pub fn set_election_tick(&mut self, election_tick: usize) {
        self.inner.election_tick = election_tick;
    }

    pub fn get_heartbeat_tick(&self) -> usize {
        self.inner.heartbeat_tick
    }

    pub fn set_heartbeat_tick(&mut self, heartbeat_tick: usize) {
        self.inner.heartbeat_tick = heartbeat_tick;
    }

    pub fn get_max_size_per_msg(&self) -> u64 {
        self.inner.max_size_per_msg
    }

    pub fn set_max_size_per_msg(&mut self, max_size_per_msg: u64) {
        self.inner.max_size_per_msg = max_size_per_msg;
    }

    pub fn get_max_inflight_msgs(&self) -> usize {
        self.inner.max_inflight_msgs
    }

    pub fn set_max_inflight_msgs(&mut self, max_inflight_msgs: usize) {
        self.inner.max_inflight_msgs = max_inflight_msgs;
    }

    pub fn get_applied(&self) -> u64 {
        self.inner.applied
    }

    pub fn set_applied(&mut self, applied: u64) {
        self.inner.applied = applied;
    }

    pub fn get_check_quorum(&self) -> bool {
        self.inner.check_quorum
    }

    pub fn set_check_quorum(&mut self, check_quorum: bool) {
        self.inner.check_quorum = check_quorum;
    }

    pub fn get_pre_vote(&self) -> bool {
        self.inner.pre_vote
    }

    pub fn set_pre_vote(&mut self, pre_vote: bool) {
        self.inner.pre_vote = pre_vote;
    }

    pub fn get_read_only_option(&self) -> PyReadOnlyOption {
        self.inner.read_only_option.into()
    }

    pub fn set_read_only_option(&mut self, read_only_option: PyReadOnlyOption) {
        self.inner.read_only_option = read_only_option.into();
    }

    pub fn get_skip_bcast_commit(&self) -> bool {
        self.inner.skip_bcast_commit
    }

    pub fn set_skip_bcast_commit(&mut self, skip_bcast_commit: bool) {
        self.inner.skip_bcast_commit = skip_bcast_commit;
    }

    pub fn get_batch_append(&self) -> bool {
        self.inner.batch_append
    }

    pub fn set_batch_append(&mut self, batch_append: bool) {
        self.inner.batch_append = batch_append;
    }

    pub fn get_priority(&self) -> i64 {
        self.inner.priority
    }

    pub fn set_priority(&mut self, priority: i64) {
        self.inner.priority = priority;
    }

    pub fn get_max_uncommitted_size(&self) -> u64 {
        self.inner.max_uncommitted_size
    }

    pub fn set_max_uncommitted_size(&mut self, max_uncommitted_size: u64) {
        self.inner.max_uncommitted_size = max_uncommitted_size;
    }

    pub fn get_max_committed_size_per_ready(&self) -> u64 {
        self.inner.max_committed_size_per_ready
    }

    pub fn set_max_committed_size_per_ready(&mut self, max_committed_size_per_ready: u64) {
        self.inner.max_committed_size_per_ready = max_committed_size_per_ready;
    }
}
