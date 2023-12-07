use raft::Config as RaftConfig;
use std::fmt;

#[derive(Clone)]
pub struct Config {
    pub raft_config: RaftConfig,
    pub log_dir: String,
    pub compacted_log_dir: String,
    pub compacted_logs_size_threshold: i32,
    pub max_retry_cnt: i32,
    pub message_timeout: f32,
    pub snapshot_interval: f32,
    pub tick_interval: f32,
    pub lmdb_map_size: i32,
    pub cluster_id: String,
    pub terminate_on_remove: bool,
}

impl Config {
    pub fn new(
        log_dir: String,
        compacted_log_dir: String,
        compacted_logs_size_threshold: i32,
        message_timeout: f32,
        max_retry_cnt: i32,
        raft_config: RaftConfig,
        snapshot_interval: f32,
        tick_interval: f32,
        lmdb_map_size: i32,
        cluster_id: String,
        terminate_on_remove: bool,
    ) -> Self {
        Self {
            raft_config,
            log_dir,
            compacted_log_dir,
            compacted_logs_size_threshold,
            max_retry_cnt,
            message_timeout,
            snapshot_interval,
            tick_interval,
            lmdb_map_size,
            cluster_id,
            terminate_on_remove,
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            raft_config: RaftConfig::default(),
            log_dir: String::from("./"),
            compacted_log_dir: String::from("./"),
            compacted_logs_size_threshold: 1024 * 1024 * 1024,
            max_retry_cnt: 2,
            message_timeout: 5.0,
            snapshot_interval: 0.0,
            tick_interval: 0.1,
            lmdb_map_size: 1024 * 1024 * 1024,
            cluster_id: String::from("default"),
            terminate_on_remove: false,
        }
    }
}

impl fmt::Debug for Config {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Config {{ \
                Raft: {{ \
                    id: {id}, \
                    election_tick: {election_tick}, \
                    heartbeat_tick: {heartbeat_tick}, \
                    applied: {applied}, \
                    max_size_per_msg: {max_size_per_msg}, \
                    max_inflight_msgs: {max_inflight_msgs}, \
                    check_quorum: {check_quorum}, \
                    pre_vote: {pre_vote}, \
                    min_election_tick: {min_election_tick}, \
                    max_election_tick: {max_election_tick}, \
                    read_only_option: {read_only_option:?}, \
                    skip_bcast_commit: {skip_bcast_commit}, \
                    batch_append: {batch_append}, \
                    priority: {priority}, \
                    max_uncommitted_size: {max_uncommitted_size}, \
                    max_committed_size_per_ready: {max_committed_size_per_ready}, \
                }}, \
                log_dir: {log_dir}, \
                compacted_log_dir: {compacted_log_dir}, \
                compacted_logs_size_threshold: {compacted_logs_size_threshold}, \
                max_retry_cnt: {max_retry_cnt}, \
                message_timeout: {message_timeout}, \
                snapshot_interval: {snapshot_interval}, \
                tick_interval: {tick_interval}, \
                lmdb_map_size: {lmdb_map_size}, \
                cluster_id: {cluster_id}, \
                terminate_on_remove: {terminate_on_remove} \
            }}",
            id = self.raft_config.id,
            election_tick = self.raft_config.election_tick,
            heartbeat_tick = self.raft_config.heartbeat_tick,
            applied = self.raft_config.applied,
            max_size_per_msg = self.raft_config.max_size_per_msg,
            max_inflight_msgs = self.raft_config.max_inflight_msgs,
            check_quorum = self.raft_config.check_quorum,
            pre_vote = self.raft_config.pre_vote,
            min_election_tick = self.raft_config.min_election_tick,
            max_election_tick = self.raft_config.max_election_tick,
            read_only_option = self.raft_config.read_only_option,
            skip_bcast_commit = self.raft_config.skip_bcast_commit,
            batch_append = self.raft_config.batch_append,
            priority = self.raft_config.priority,
            max_uncommitted_size = self.raft_config.max_uncommitted_size,
            max_committed_size_per_ready = self.raft_config.max_committed_size_per_ready,
            log_dir = self.log_dir,
            compacted_log_dir = self.compacted_log_dir,
            compacted_logs_size_threshold = self.compacted_logs_size_threshold,
            max_retry_cnt = self.max_retry_cnt,
            message_timeout = self.message_timeout,
            snapshot_interval = self.snapshot_interval,
            tick_interval = self.tick_interval,
            lmdb_map_size = self.lmdb_map_size,
            cluster_id = self.cluster_id,
            terminate_on_remove = self.terminate_on_remove,
        )
    }
}
