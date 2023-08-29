import abc
from dataclasses import dataclass
from typing import Optional

from raftify.utils import PickleSerializer


class RaftResponse(metaclass=abc.ABCMeta):
    pass


@dataclass
class RaftRespWrongLeader(RaftResponse, PickleSerializer):
    leader_id: int
    leader_addr: str


@dataclass
class RaftRespJoinSuccess(RaftResponse, PickleSerializer):
    assigned_id: int
    peer_addrs: dict[int, str]


@dataclass
class RaftRespIdReserved(RaftResponse, PickleSerializer):
    leader_id: int
    reserved_id: int
    peer_addrs: dict[int, str]


@dataclass
class RaftRespResponse(RaftResponse, PickleSerializer):
    data: bytes


@dataclass
class RaftRespError(RaftResponse, PickleSerializer):
    data: Optional[bytes] = None


@dataclass
class RaftRespOk(RaftResponse, PickleSerializer):
    pass
