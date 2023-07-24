import pickle
from multiprocessing import Queue
from typing import Dict

from riteraft.protos.eraftpb_pb2 import ConfChange, Message


class Encoder:
    def encode(self):
        return pickle.dumps(self)


class RaftRespWrongLeader(Encoder):
    def __init__(self, leader_id: int, leader_addr: str):
        self.leader_id = leader_id
        self.leader_addr = leader_addr

    @classmethod
    def decode(cls, data: bytes) -> "RaftRespWrongLeader":
        return cls(pickle.loads(data))


class RaftRespJoinSuccess(Encoder):
    def __init__(self, assigned_id: int, peer_addrs: Dict[int, str]):
        self.assigned_id = assigned_id
        self.peer_addrs = peer_addrs

    @classmethod
    def decode(cls, data: bytes) -> "RaftRespJoinSuccess":
        return cls(pickle.loads(data))


class RaftRespIdReserved(Encoder):
    def __init__(self, leader_id: int, reserved_id: int, peer_addrs: Dict[int, str]):
        self.leader_id = leader_id
        self.reserved_id = reserved_id
        self.peer_addrs = peer_addrs

    @classmethod
    def decode(cls, data: bytes) -> "RaftRespIdReserved":
        return cls(pickle.loads(data))


class RaftRespResponse(Encoder):
    def __init__(self, data: bytes):
        self.data = data

    @classmethod
    def decode(cls, data: bytes) -> "RaftRespResponse":
        return cls(pickle.loads(data))


class RaftRespError(Encoder):
    def __init__(self):
        pass

    @classmethod
    def decode(cls, data: bytes) -> "RaftRespError":
        return cls(pickle.loads(data))


class RaftRespOk(Encoder):
    def __init__(self):
        pass

    @classmethod
    def decode(cls, data: bytes) -> "RaftRespOk":
        return cls(pickle.loads(data))


class MessagePropose(Encoder):
    def __init__(self, proposal: bytes, chan: Queue):
        self.proposal = proposal
        self.chan = chan

    @classmethod
    def decode(cls, data: bytes) -> "MessagePropose":
        return cls(pickle.loads(data))


class MessageConfigChange(Encoder):
    def __init__(self, change: ConfChange, chan: Queue):
        self.change = change
        self.chan = chan

    @classmethod
    def decode(cls, data: bytes) -> "MessageConfigChange":
        return cls(pickle.loads(data))


class MessageRequestId(Encoder):
    def __init__(self, addr: str, chan: Queue):
        self.addr = addr
        self.chan = chan

    @classmethod
    def decode(cls, data: bytes) -> "MessageRequestId":
        return cls(pickle.loads(data))


class MessageReportUnreachable(Encoder):
    def __init__(self, node_id: int):
        self.node_id = node_id

    @classmethod
    def decode(cls, data: bytes) -> "MessageReportUnreachable":
        return cls(pickle.loads(data))


class MessageRaft(Encoder):
    def __init__(self, msg: Message):
        self.msg = msg

    @classmethod
    def decode(cls, data: bytes) -> "MessageRaft":
        return cls(pickle.loads(data))
