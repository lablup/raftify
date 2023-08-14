from rraft import Config


class RaftConfig:
    def __init__(self, cfg: Config | None) -> None:
        self.config = cfg or Config.default()

    @staticmethod
    def from_dict(cfg_dict: dict) -> "RaftConfig":
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
