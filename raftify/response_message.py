import abc
from dataclasses import dataclass
from typing import Optional

from .utils import PickleSerializer


class RaftResponse(metaclass=abc.ABCMeta):
    pass


@dataclass
class WrongLeaderRespMessage(RaftResponse, PickleSerializer):
    leader_id: int
    leader_addr: str


@dataclass
class JoinSuccessRespMessage(RaftResponse, PickleSerializer):
    assigned_id: int
    peers: bytes


@dataclass
class IdReservedRespMessage(RaftResponse, PickleSerializer):
    leader_id: int
    reserved_id: int
    peers: bytes


@dataclass
class RaftRespMessage(RaftResponse, PickleSerializer):
    data: bytes


@dataclass
class RaftErrorRespMessage(RaftResponse, PickleSerializer):
    data: Optional[bytes] = None


@dataclass
class RaftOkRespMessage(RaftResponse, PickleSerializer):
    pass


@dataclass
class DebugNodeResponse(RaftResponse, PickleSerializer):
    result: dict


@dataclass
class DebugEntriesResponse(RaftResponse, PickleSerializer):
    result: dict
