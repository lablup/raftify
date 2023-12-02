use raft::Config as RaftConfig;

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
