from dataclasses import dataclass

from riteraft.utils import PickleSerializer


@dataclass
class RaftRespWrongLeader(PickleSerializer):
    leader_id: int
    leader_addr: str


@dataclass
class RaftRespJoinSuccess(PickleSerializer):
    assigned_id: int
    peer_addrs: dict[int, str]


@dataclass
class RaftRespIdReserved(PickleSerializer):
    leader_id: int
    reserved_id: int
    peer_addrs: dict[int, str]


@dataclass
class RaftRespResponse(PickleSerializer):
    data: bytes


@dataclass
class RaftRespError(PickleSerializer):
    pass


@dataclass
class RaftRespOk(PickleSerializer):
    pass
