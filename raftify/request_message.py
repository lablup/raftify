import abc
from dataclasses import dataclass
from multiprocessing import Queue
from typing import Optional

from raftify.protos import eraftpb_pb2, raft_service_pb2
from raftify.utils import PickleSerializer


class RaftRequest(metaclass=abc.ABCMeta):
    pass


@dataclass
class ProposeReqMessage(RaftRequest, PickleSerializer):
    data: bytes
    chan: Queue


@dataclass
class RerouteToLeaderReqMessage(RaftRequest, PickleSerializer):
    proposed_data: Optional[bytes]
    confchange: Optional[raft_service_pb2.ConfChange]
    type: raft_service_pb2.RerouteMsgType
    chan: Queue


@dataclass
class ConfigChangeReqMessage(RaftRequest, PickleSerializer):
    change: eraftpb_pb2.ConfChange
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
