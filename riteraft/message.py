import pickle
from multiprocessing import Queue
from typing import Dict

from rraft import ConfChange, Message


class Serializable:
    def dumps(self) -> bytes:
        return pickle.dumps(self)


class RaftRespWrongLeader(Serializable):
    def __init__(self, leader_id: int, leader_addr: str):
        self.leader_id = leader_id
        self.leader_addr = leader_addr


class RaftRespJoinSuccess(Serializable):
    def __init__(self, assigned_id: int, peer_addrs: Dict[int, str]):
        self.assigned_id = assigned_id
        self.peer_addrs = peer_addrs


class RaftRespIdReserved(Serializable):
    def __init__(self, id: int):
        self.id = id


class RaftRespResponse(Serializable):
    def __init__(self, data: bytes):
        self.data = data


class RaftRespError(Serializable):
    def __init__(self):
        pass


class RaftRespOk(Serializable):
    def __init__(self):
        pass


RaftResponse = (
    RaftRespWrongLeader
    | RaftRespJoinSuccess
    | RaftRespIdReserved
    | RaftRespResponse
    | RaftRespError
    | RaftRespOk
)


class MessagePropose(Serializable):
    def __init__(self, proposal: bytes, chan: Queue):
        self.proposal = proposal
        self.chan = chan


class MessageConfigChange(Serializable):
    def __init__(self, change: ConfChange, chan: Queue):
        self.change = change
        self.chan = chan


class MessageRequestId(Serializable):
    def __init__(self, chan: Queue):
        self.chan = chan


class MessageReportUnreachable(Serializable):
    def __init__(self, node_id: int):
        self.node_id = node_id


class MessageRaft(Serializable):
    def __init__(self, msg: Message):
        self.msg = msg


Message = (
    MessagePropose
    | MessageConfigChange
    | MessageRequestId
    | MessageReportUnreachable
    | MessageRaft
)
