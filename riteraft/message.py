import pickle
from multiprocessing import Queue
from typing import Dict

from riteraft.protos.eraftpb_pb2 import ConfChange, Message


class Decoder:
    def decode(self):
        return pickle.dumps(self)


class RaftRespWrongLeader(Decoder):
    def __init__(self, leader_id: int, leader_addr: str):
        self.leader_id = leader_id
        self.leader_addr = leader_addr

    @classmethod
    def encode(cls, data: bytes) -> "RaftRespWrongLeader":
        return cls(pickle.loads(data))


class RaftRespJoinSuccess(Decoder):
    def __init__(self, assigned_id: int, peer_addrs: Dict[int, str]):
        self.assigned_id = assigned_id
        self.peer_addrs = peer_addrs

    @classmethod
    def encode(cls, data: bytes) -> "RaftRespJoinSuccess":
        return cls(pickle.loads(data))


class RaftRespIdReserved(Decoder):
    def __init__(self, id: int):
        self.id = id

    @classmethod
    def encode(cls, data: bytes) -> "RaftRespIdReserved":
        return cls(pickle.loads(data))


class RaftRespResponse(Decoder):
    def __init__(self, data: bytes):
        self.data = data

    @classmethod
    def encode(cls, data: bytes) -> "RaftRespResponse":
        return cls(pickle.loads(data))


class RaftRespError(Decoder):
    def __init__(self):
        pass

    @classmethod
    def encode(cls, data: bytes) -> "RaftRespError":
        return cls(pickle.loads(data))


class RaftRespOk(Decoder):
    def __init__(self):
        pass

    @classmethod
    def encode(cls, data: bytes) -> "RaftRespOk":
        return cls(pickle.loads(data))


class MessagePropose(Decoder):
    def __init__(self, proposal: bytes, chan: Queue):
        self.proposal = proposal
        self.chan = chan

    @classmethod
    def encode(cls, data: bytes) -> "MessagePropose":
        return cls(pickle.loads(data))


class MessageConfigChange(Decoder):
    def __init__(self, change: ConfChange, chan: Queue):
        self.change = change
        self.chan = chan

    @classmethod
    def encode(cls, data: bytes) -> "MessageConfigChange":
        return cls(pickle.loads(data))


class MessageRequestId(Decoder):
    def __init__(self, chan: Queue):
        self.chan = chan

    @classmethod
    def encode(cls, data: bytes) -> "MessageRequestId":
        return cls(pickle.loads(data))


class MessageReportUnreachable(Decoder):
    def __init__(self, node_id: int):
        self.node_id = node_id

    @classmethod
    def encode(cls, data: bytes) -> "MessageReportUnreachable":
        return cls(pickle.loads(data))


class MessageRaft(Decoder):
    def __init__(self, msg: Message):
        self.msg = msg

    @classmethod
    def encode(cls, data: bytes) -> "MessageRaft":
        return cls(pickle.loads(data))
