import abc
from dataclasses import dataclass
from multiprocessing import Queue
from typing import Optional

from raftify.protos import eraftpb_pb2, raft_service_pb2
from raftify.utils import PickleSerializer


class RaftRequest(metaclass=abc.ABCMeta):
    pass


@dataclass
class MessagePropose(RaftRequest, PickleSerializer):
    data: bytes
    chan: Queue


@dataclass
class MessageRerouteToLeader(RaftRequest, PickleSerializer):
    proposed_data: Optional[bytes]
    confchange: Optional[raft_service_pb2.ConfChange]
    type: raft_service_pb2.RerouteMsgType
    chan: Queue


@dataclass
class MessageConfigChange(RaftRequest, PickleSerializer):
    change: eraftpb_pb2.ConfChange
    chan: Queue


@dataclass
class MessageRequestId(RaftRequest, PickleSerializer):
    addr: str
    chan: Queue


@dataclass
class MessageReportUnreachable(RaftRequest, PickleSerializer):
    node_id: int


@dataclass
class MessageRaft(RaftRequest, PickleSerializer):
    msg: eraftpb_pb2.Message
