import abc
from asyncio import Queue
from dataclasses import dataclass
from typing import Optional

from .protos import eraftpb_pb2, raft_service_pb2


class RequestMessage(metaclass=abc.ABCMeta):
    """
    Used for transferring data from the `RaftServer.run` coroutine to the `RaftNode.run` coroutine.
    """

    pass


@dataclass
class ProposeReqMessage(RequestMessage):
    """
    Requests used for proposing to the Raft cluster.
    """

    data: bytes
    response_chan: Queue


@dataclass
class MemberBootstrapReadyReqMessage(RequestMessage):
    """
    Requests used by a member node to notify the leader node that it is ready to boot.
    """

    follower_id: int
    response_chan: Queue


@dataclass
class ClusterBootstrapReadyReqMessage(RequestMessage):
    """
    Requests used by the leader node to notify the member nodes that the cluster is ready to boot.
    """

    peers: bytes
    response_chan: Queue


@dataclass
class RequestIdReqMessage(RequestMessage):
    """
    Requests used by dynamically added nodes to receive the new node_id assignments.
    """

    addr: str
    response_chan: Queue


@dataclass
class ConfigChangeReqMessage(RequestMessage):
    """
    Requests used for proposing new conf change.
    """

    conf_change: eraftpb_pb2.ConfChangeV2
    response_chan: Queue


@dataclass
class RerouteToLeaderReqMessage(RequestMessage):
    """
    Requests used when a follower node reroutes a received request to the leader node
    """

    proposed_data: Optional[bytes]
    conf_change: Optional[eraftpb_pb2.ConfChangeV2]
    type: raft_service_pb2.RerouteMsgType
    response_chan: Queue


@dataclass
class ApplyConfigChangeForcelyReqMessage(RequestMessage):
    """ """

    conf_change: eraftpb_pb2.ConfChangeV2
    response_chan: Queue


@dataclass
class ReportUnreachableReqMessage(RequestMessage):
    """ """

    node_id: int


@dataclass
class RaftReqMessage(RequestMessage):
    """ """

    msg: eraftpb_pb2.Message


@dataclass
class DebugNodeReqMessage(RequestMessage):
    """ """

    response_chan: Queue


@dataclass
class DebugEntriesReqMessage(RequestMessage):
    """ """

    response_chan: Queue


@dataclass
class VersionReqMessage(RequestMessage):
    """ """

    response_chan: Queue


@dataclass
class GetPeersReqMessage(RequestMessage):
    """ """

    response_chan: Queue
