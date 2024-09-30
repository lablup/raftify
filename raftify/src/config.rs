use std::fmt;

use crate::{error::Error, raft::Config as RaftConfig, InitialRole, Peers, Result};

#[derive(Clone)]
pub struct Config {
    pub raft_config: RaftConfig,
    pub log_dir: String,

    pub bootstrap_from_snapshot: bool,
    pub save_compacted_logs: bool,
    pub compacted_log_dir: String,
    pub compacted_log_size_threshold: u64,

    pub tick_interval: f32,
    pub lmdb_map_size: u64,
    pub cluster_id: String,
    pub conf_change_request_timeout: f32,

    pub initial_peers: Option<Peers>,
    pub snapshot_interval: Option<f32>,
}

impl Config {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        log_dir: String,
        save_compacted_logs: bool,
        compacted_log_dir: String,
        compacted_log_size_threshold: u64,
        raft_config: RaftConfig,
        tick_interval: f32,
        lmdb_map_size: u64,
        cluster_id: String,
        bootstrap_from_snapshot: bool,
        conf_change_request_timeout: f32,
        initial_peers: Option<Peers>,
        snapshot_interval: Option<f32>,
    ) -> Self {
        Self {
            raft_config,
            log_dir,
            save_compacted_logs,
            compacted_log_dir,
            compacted_log_size_threshold,
            snapshot_interval,
            tick_interval,
            lmdb_map_size,
            initial_peers,
            cluster_id,
            conf_change_request_timeout,
            bootstrap_from_snapshot,
        }
    }
}

impl Config {
    pub fn validate(&self) -> Result<()> {
        if self.initial_peers.is_some() {
            let leaders = self
                .initial_peers
                .clone()
                .unwrap()
                .inner
                .into_iter()
                .filter(|(_, peer)| peer.role == InitialRole::Leader)
                .map(|(key, _)| key)
                .collect::<Vec<_>>();

            if leaders.len() > 1 {
                return Err(Error::ConfigInvalid(
                    "initial_peers should contain at most 1 leaders".to_owned(),
                ));
            }
        }

        self.raft_config.validate()?;
        Ok(())
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            raft_config: RaftConfig::default(),
            log_dir: String::from("./"),
            save_compacted_logs: false,
            compacted_log_dir: String::from("./"),
            compacted_log_size_threshold: 1024 * 1024 * 1024,
            tick_interval: 0.1,
            lmdb_map_size: 1024 * 1024 * 1024,
            cluster_id: String::from("default"),
            conf_change_request_timeout: 2.0,
            initial_peers: None,
            snapshot_interval: None,
            bootstrap_from_snapshot: false,
        }
    }
}

impl fmt::Debug for Config {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Config {{ \
                raft_config: {{ \
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
                bootstrap_from_snapshot: {bootstrap_from_snapshot}, \
                save_compacted_logs: {save_compacted_logs}, \
                compacted_log_dir: {compacted_log_dir}, \
                compacted_log_size_threshold: {compacted_log_size_threshold}, \
                snapshot_interval: {snapshot_interval:?}, \
                tick_interval: {tick_interval}, \
                initial_peers: {initial_peers:?}, \
                lmdb_map_size: {lmdb_map_size}, \
                cluster_id: {cluster_id}, \
                conf_change_request_timeout: {conf_change_request_timeout}, \
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
            save_compacted_logs = self.save_compacted_logs,
            compacted_log_dir = self.compacted_log_dir,
            compacted_log_size_threshold = self.compacted_log_size_threshold,
            snapshot_interval = self.snapshot_interval,
            tick_interval = self.tick_interval,
            lmdb_map_size = self.lmdb_map_size,
            initial_peers = self.initial_peers,
            cluster_id = self.cluster_id,
            conf_change_request_timeout = self.conf_change_request_timeout,
            bootstrap_from_snapshot = self.bootstrap_from_snapshot,
        )
    }
}
