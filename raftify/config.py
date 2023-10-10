from dataclasses import dataclass

from rraft import Config


@dataclass
class RaftifyConfig:
    """
    Raft Configurations.

    Attributes:
    - config: Configuration object. Uses default settings if not provided.
    - log_dir: Directory path where log files are stored.
    - max_retry_cnt: Maximum number of retries for a request.
    - auto_remove_node: Whether to automatically remove a node from the cluster if it keeps not responding.
    - connection_fail_limit: Maximum number of connection failures before removing a node from the cluster.
    - message_timeout: Timeout duration for a message request.
    - snapshot_interval: Interval between snapshots.
        Set to 0 to disable.
        Snapshots are also created after configuration changes are applied.
    - lmdb_map_size: Maximum size lmdb database may grow to.
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

    raft_config: Config

    log_dir: str

    max_retry_cnt: int

    message_timeout: float

    auto_remove_node: bool

    connection_fail_limit: int

    snapshot_interval: float

    tick_interval: float

    lmdb_map_size: int

    def __init__(
        self,
        *,
        log_dir: str = "./",
        max_retry_cnt: int = 5,
        message_timeout: float = 5.0,
        auto_remove_node: bool = True,
        connection_fail_limit: int = 5,
        raft_config: Config = Config.default(),
        snapshot_interval: float = 0.0,
        tick_interval: float = 0.1,
        lmdb_map_size: int = 1024 * 1024 * 1024,
    ) -> None:
        self.log_dir = log_dir
        self.max_retry_cnt = max_retry_cnt
        self.message_timeout = message_timeout
        self.auto_remove_node = auto_remove_node
        self.connection_fail_limit = connection_fail_limit
        self.snapshot_interval = snapshot_interval
        self.tick_interval = tick_interval
        self.lmdb_map_size = lmdb_map_size
        self.raft_config = raft_config or Config.default()

    @staticmethod
    def new_raft_config(cfg_dict: dict) -> "Config":
        cfg = Config.default()

        for key in RaftifyConfig.raft_config_keys:
            if key in cfg_dict:
                if cfg_dict[key] is not None:
                    getattr(cfg, "set_" + key)(cfg_dict[key])

        return cfg
