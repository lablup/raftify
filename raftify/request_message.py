import abc
from asyncio import Queue
from dataclasses import dataclass
from typing import Optional

from .protos import eraftpb_pb2, raft_service_pb2
from .utils import PickleSerializer


class RaftRequest(metaclass=abc.ABCMeta):
    pass


@dataclass
class ProposeReqMessage(RaftRequest, PickleSerializer):
    data: bytes
    chan: Queue


@dataclass
class MemberBootstrapReadyReqMessage(RaftRequest, PickleSerializer):
    follower_id: int
    chan: Queue


@dataclass
class ClusterBootstrapReadyReqMessage(RaftRequest, PickleSerializer):
    peers: bytes
    chan: Queue


@dataclass
class RerouteToLeaderReqMessage(RaftRequest, PickleSerializer):
    proposed_data: Optional[bytes]
    conf_change: Optional[eraftpb_pb2.ConfChangeV2]
    type: raft_service_pb2.RerouteMsgType
    chan: Queue


@dataclass
class ConfigChangeReqMessage(RaftRequest, PickleSerializer):
    conf_change: eraftpb_pb2.ConfChangeV2
    chan: Queue


@dataclass
class ApplyConfigChangeForcelyReqMessage(RaftRequest, PickleSerializer):
    conf_change: eraftpb_pb2.ConfChangeV2
    chan: Queue


@dataclass
class RequestIdReqMessage(RaftRequest, PickleSerializer):
    addr: str
    chan: Queue


@dataclass
class ReportUnreachableReqMessage(RaftRequest, PickleSerializer):
    node_id: int


@dataclass
class RaftReqMessage(RaftRequest, PickleSerializer):
    msg: eraftpb_pb2.Message


@dataclass
class DebugNodeRequest(RaftRequest, PickleSerializer):
    chan: Queue


@dataclass
class DebugEntriesRequest(RaftRequest, PickleSerializer):
    chan: Queue


@dataclass
class VersionRequest(RaftRequest, PickleSerializer):
    chan: Queue
