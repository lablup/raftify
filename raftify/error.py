class UnknownError(Exception):
    pass


class LeaderNotFoundError(Exception):
    """
    Raise when the 'request_id' fails to find the leader node in the cluster.
    """

    def __init__(
        self,
        message="Leader not found in the cluster. Check your Raft configuration and make sure that any of them is alive.",
    ):
        super().__init__(message)


class ClusterJoinError(Exception):
    """
    Raise when the 'join_cluster' fails for some reason.
    """

    def __init__(self, cause=None):
        self.cause = cause
        super().__init__(str(cause))


class ClusterBootstrapError(Exception):
    """ """

    def __init__(self, cause=None):
        self.cause = cause
        super().__init__(str(cause))


class ProposalRejectError(Exception):
    """ """

    def __init__(self, cause=None):
        self.cause = cause
        super().__init__(str(cause))
