class UnknownError(Exception):
    pass


class LeaderNotFoundError(Exception):
    def __init__(
        self,
        message="Leader not found in the cluster. Check your Raft configuration and check to make sure that any of them is alive.",
    ):
        super().__init__(message)


class ClusterJoinError(Exception):
    def __init__(self, cause=None):
        self.cause = cause
        super().__init__(str(cause))
