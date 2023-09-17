from rraft import Config

from dataclasses import dataclass


@dataclass
class RaftifyConfig:
    """
    Raft Configurations.

    Attributes:
    - config: Configuration object. Uses default settings if not provided.
    - log_dir: Directory path where log files are stored.
    - use_log_compaction: Whether to use log compaction. True if used, otherwise False.
    - max_retry_cnt: Maximum number of retries for a request.
    - auto_remove_node: Whether to automatically remove a node from the cluster if it keeps not responding.
    - connection_fail_limit: Maximum number of connection failures before removing a node from the cluster.
    - message_timeout: Timeout duration for a message request.
    - lmdb_map_size: Maximum size lmdb database may grow to.
    - no_restoration: Ignore previous logs when bootstrapping the cluster. Useful for testing.
    """

    raft_config_keys = [
        "election_tick",
        "min_election_tick",
        "max_election_tick",
        "heartbeat_tick",
        "max_committed_size_per_ready",
        "max_size_per_msg",
        "max_inflight_msgs",
        "check_quorum",
        "batch_append",
        "max_uncommitted_size",
        "pre_vote",
        "priority",
        "applied",
        "skip_bcast_commit",
    ]

    config: Config

    log_dir: str

    use_log_compaction: bool

    max_retry_cnt: int

    message_timeout: int

    auto_remove_node: bool

    connection_fail_limit: int

    snapshot_interval: float

    tick_interval: float

    lmdb_map_size: int

    no_restoration: bool

    def __init__(
        self,
        *,
        log_dir: str = "./",
        max_retry_cnt: int = 5,
        message_timeout: float = 0.1,
        auto_remove_node: bool = True,
        connection_fail_limit: int = 5,
        use_log_compaction: bool = False,
        config: Config = Config.default(),
        snapshot_interval: float = 15.0,
        tick_interval: float = 0.1,
        lmdb_map_size: int = 1024 * 1024 * 1024,
        no_restoration: bool = False,
    ) -> None:
        self.log_dir = log_dir
        self.use_log_compaction = use_log_compaction
        self.max_retry_cnt = max_retry_cnt
        self.message_timeout = message_timeout
        self.auto_remove_node = auto_remove_node
        self.connection_fail_limit = connection_fail_limit
        self.snapshot_interval = snapshot_interval
        self.tick_interval = tick_interval
        self.lmdb_map_size = lmdb_map_size
        self.no_restoration = no_restoration
        self.config = config or Config.default()

    @staticmethod
    def new_raft_config(cfg_dict: dict) -> "Config":
        cfg = Config.default()

        for key in RaftifyConfig.raft_config_keys:
            if key in cfg_dict:
                if cfg_dict[key] is not None:
                    getattr(cfg, "set_" + key)(cfg_dict[key])

        return cfg
