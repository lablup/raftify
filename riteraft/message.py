from multiprocessing import Queue
from typing import Dict

from rraft import ConfChange, Message


class RaftRespWrongLeader:
    def __init__(self, leader_id: int, leader_addr: str):
        self.leader_id = leader_id
        self.leader_addr = leader_addr


class RaftRespJoinSuccess:
    def __init__(self, assigned_id: int, peer_addrs: Dict[int, str]):
        self.assigned_id = assigned_id
        self.peer_addrs = peer_addrs


class RaftRespIdReserved:
    def __init__(self, id: int):
        self.id = id


class RaftRespResponse:
    def __init__(self, data: bytes):
        self.data = data


class RaftRespError:
    def __init__(self):
        pass


class RaftRespOk:
    def __init__(self):
        pass


class MessagePropose:
    def __init__(self, proposal: bytes, chan: Queue):
        self.proposal = proposal
        self.chan = chan


class MessageConfigChange:
    def __init__(self, change: ConfChange, chan: Queue):
        self.change = change
        self.chan = chan


class MessageRequestId:
    def __init__(self, chan: Queue):
        self.chan = chan


class MessageReportUnreachable:
    def __init__(self, node_id: int):
        self.node_id = node_id


class MessageRaft:
    def __init__(self, msg: Message):
        self.msg = msg
