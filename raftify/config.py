from rraft import Config


class RaftConfig:
    """
    Raft Configuration class.

    Attributes:
    - config: Configuration object. Uses default settings if not provided.
    - log_dir: Directory path where log files are stored.
    - use_log_compaction: Whether to use log compaction. True if used, otherwise False.
    """

    config: Config

    log_dir: str

    use_log_compaction: bool

    def __init__(
        self,
        *,
        log_dir: str = "./",
        use_log_compaction: bool = False,
        config: Config = Config.default()
    ) -> None:
        """
        :param log_dir: Directory path where log files are stored.
        :param use_log_compaction: Whether to use log compaction. True if used, otherwise False.
        :param config: Configuration object. Uses default settings if not provided.
        """
        self.log_dir = log_dir
        self.use_log_compaction = use_log_compaction
        self.config = config or Config.default()

    @staticmethod
    def new_raft_config(cfg_dict: dict) -> "Config":
        cfg = Config.default()

        for key in [
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
        ]:
            if key in cfg_dict:
                getattr(cfg, "set_" + key)(cfg_dict[key])

        return cfg
