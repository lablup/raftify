import abc
from dataclasses import dataclass
from typing import Optional

from .utils import PickleSerializer


class ResponseMessage(metaclass=abc.ABCMeta):
    """
    ResponseMessages are passed from the RaftNode to the RaftServer via response_queue.
    Note that ResponseMessage must be passed to prevent a TimeoutError,
    although the timing of its delivery varies depending on the type of message.
    """

    pass


@dataclass
class WrongLeaderRespMessage(ResponseMessage, PickleSerializer):
    leader_id: int
    leader_addr: str


@dataclass
class JoinSuccessRespMessage(ResponseMessage, PickleSerializer):
    assigned_id: int
    raw_peers: bytes


@dataclass
class RemovedPeerSuccessMessage(ResponseMessage, PickleSerializer):
    pass


@dataclass
class ConfChangeRejectMessage(ResponseMessage, PickleSerializer):
    pass


@dataclass
class IdReservedRespMessage(ResponseMessage, PickleSerializer):
    leader_id: int
    reserved_id: int
    raw_peers: bytes


@dataclass
class RaftRespMessage(ResponseMessage, PickleSerializer):
    data: bytes
    rejected: bool = False


@dataclass
class RaftErrorRespMessage(ResponseMessage, PickleSerializer):
    data: Optional[bytes]


@dataclass
class ClusterBootstrapReadyRespMessage(ResponseMessage, PickleSerializer):
    pass


@dataclass
class SendMessageRespMessage(ResponseMessage, PickleSerializer):
    pass


@dataclass
class PeerRemovalSuccessRespMessage(ResponseMessage, PickleSerializer):
    pass


@dataclass
class MemberBootstrapReadyRespMessage(ResponseMessage, PickleSerializer):
    pass


@dataclass
class DebugNodeResponse(ResponseMessage, PickleSerializer):
    result: dict


@dataclass
class DebugEntriesResponse(ResponseMessage, PickleSerializer):
    result: dict


@dataclass
class VersionResponse(ResponseMessage, PickleSerializer):
    result: str


@dataclass
class GetPeersResponse(ResponseMessage, PickleSerializer):
    raw_peers: bytes
