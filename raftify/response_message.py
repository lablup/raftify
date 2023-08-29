import abc
from dataclasses import dataclass
from typing import Optional

from raftify.utils import PickleSerializer


class RaftResponse(metaclass=abc.ABCMeta):
    pass


@dataclass
class WrongLeaderRespMessage(RaftResponse, PickleSerializer):
    leader_id: int
    leader_addr: str


@dataclass
class JoinSuccessRespMessage(RaftResponse, PickleSerializer):
    assigned_id: int
    peer_addrs: dict[int, str]


@dataclass
class IdReservedRespMessage(RaftResponse, PickleSerializer):
    leader_id: int
    reserved_id: int
    peer_addrs: dict[int, str]


@dataclass
class RaftRespMessage(RaftResponse, PickleSerializer):
    data: bytes


@dataclass
class RaftErrorRespMessage(RaftResponse, PickleSerializer):
    data: Optional[bytes] = None


@dataclass
class RaftOkRespMessage(RaftResponse, PickleSerializer):
    pass
