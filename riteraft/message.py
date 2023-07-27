from dataclasses import dataclass
import pickle
from multiprocessing import Queue

from riteraft.protos.eraftpb_pb2 import ConfChange, Message


class Serializer:
    def encode(self):
        return pickle.dumps(self)

    @classmethod
    def decode(cls, data: bytes):
        return cls(*pickle.loads(data))


@dataclass
class RaftRespWrongLeader(Serializer):
    leader_id: int
    leader_addr: str


@dataclass
class RaftRespJoinSuccess(Serializer):
    assigned_id: int
    peer_addrs: dict[int, str]


@dataclass
class RaftRespIdReserved(Serializer):
    leader_id: int
    reserved_id: int
    peer_addrs: dict[int, str]


@dataclass
class RaftRespResponse(Serializer):
    data: bytes


@dataclass
class RaftRespError(Serializer):
    def __init__(self):
        pass


@dataclass
class RaftRespOk(Serializer):
    def __init__(self):
        pass


@dataclass
class MessagePropose(Serializer):
    proposal: bytes
    chan: Queue


@dataclass
class MessageConfigChange(Serializer):
    change: ConfChange
    chan: Queue


@dataclass
class MessageRequestId(Serializer):
    addr: str
    chan: Queue


@dataclass
class MessageReportUnreachable(Serializer):
    node_id: int


@dataclass
class MessageRaft(Serializer):
    msg: Message
