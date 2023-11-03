import abc
from asyncio import Queue
from dataclasses import dataclass
from typing import Optional

from .protos import eraftpb_pb2, raft_service_pb2
from .utils import PickleSerializer


class RequestMessage(metaclass=abc.ABCMeta):
    """
    RequestMessages are passed from the RaftServer to the RaftNode via message_queue.
    """

    pass


@dataclass
class ProposeReqMessage(RequestMessage, PickleSerializer):
    data: bytes
    chan: Queue


@dataclass
class MemberBootstrapReadyReqMessage(RequestMessage, PickleSerializer):
    follower_id: int
    chan: Queue


@dataclass
class ClusterBootstrapReadyReqMessage(RequestMessage, PickleSerializer):
    peers: bytes
    chan: Queue


@dataclass
class RequestIdReqMessage(RequestMessage, PickleSerializer):
    addr: str
    chan: Queue


@dataclass
class ConfigChangeReqMessage(RequestMessage, PickleSerializer):
    conf_change: eraftpb_pb2.ConfChangeV2
    chan: Queue


@dataclass
class RerouteToLeaderReqMessage(RequestMessage, PickleSerializer):
    proposed_data: Optional[bytes]
    conf_change: Optional[eraftpb_pb2.ConfChangeV2]
    type: raft_service_pb2.RerouteMsgType
    chan: Queue


@dataclass
class ApplyConfigChangeForcelyReqMessage(RequestMessage, PickleSerializer):
    conf_change: eraftpb_pb2.ConfChangeV2
    chan: Queue


@dataclass
class ReportUnreachableReqMessage(RequestMessage, PickleSerializer):
    node_id: int


# TODO: Rename this weird type name to something more meaningful
@dataclass
class RaftReqMessage(RequestMessage, PickleSerializer):
    msg: eraftpb_pb2.Message


@dataclass
class DebugNodeReqMessage(RequestMessage, PickleSerializer):
    chan: Queue


@dataclass
class DebugEntriesReqMessage(RequestMessage, PickleSerializer):
    chan: Queue


@dataclass
class VersionReqMessage(RequestMessage, PickleSerializer):
    chan: Queue
