from dataclasses import dataclass
import pickle
from multiprocessing import Queue

from riteraft.protos.eraftpb_pb2 import ConfChange, Message


class Encoder:
    def encode(self):
        return pickle.dumps(self)


@dataclass
class RaftRespWrongLeader(Encoder):
    leader_id: int
    leader_addr: str

    @classmethod
    def decode(cls, data: bytes) -> "RaftRespWrongLeader":
        return cls(pickle.loads(data))


@dataclass
class RaftRespJoinSuccess(Encoder):
    assigned_id: int
    peer_addrs: dict[int, str]

    @classmethod
    def decode(cls, data: bytes) -> "RaftRespJoinSuccess":
        return cls(pickle.loads(data))


@dataclass
class RaftRespIdReserved(Encoder):
    leader_id: int
    reserved_id: int
    peer_addrs: dict[int, str]

    @classmethod
    def decode(cls, data: bytes) -> "RaftRespIdReserved":
        return cls(pickle.loads(data))


@dataclass
class RaftRespResponse(Encoder):
    data: bytes

    @classmethod
    def decode(cls, data: bytes) -> "RaftRespResponse":
        return cls(pickle.loads(data))


@dataclass
class RaftRespError(Encoder):
    def __init__(self):
        pass

    @classmethod
    def decode(cls, data: bytes) -> "RaftRespError":
        return cls(pickle.loads(data))


@dataclass
class RaftRespOk(Encoder):
    def __init__(self):
        pass

    @classmethod
    def decode(cls, data: bytes) -> "RaftRespOk":
        return cls(pickle.loads(data))


@dataclass
class MessagePropose(Encoder):
    proposal: bytes
    chan: Queue

    @classmethod
    def decode(cls, data: bytes) -> "MessagePropose":
        return cls(pickle.loads(data))


@dataclass
class MessageConfigChange(Encoder):
    change: ConfChange
    chan: Queue

    @classmethod
    def decode(cls, data: bytes) -> "MessageConfigChange":
        return cls(pickle.loads(data))


@dataclass
class MessageRequestId(Encoder):
    addr: str
    chan: Queue

    @classmethod
    def decode(cls, data: bytes) -> "MessageRequestId":
        return cls(pickle.loads(data))


@dataclass
class MessageReportUnreachable(Encoder):
    node_id: int

    @classmethod
    def decode(cls, data: bytes) -> "MessageReportUnreachable":
        return cls(pickle.loads(data))


@dataclass
class MessageRaft(Encoder):
    msg: Message

    @classmethod
    def decode(cls, data: bytes) -> "MessageRaft":
        return cls(pickle.loads(data))
