import abc
from dataclasses import dataclass
from typing import Optional


class ResponseMessage(metaclass=abc.ABCMeta):
    """
    ResponseMessages are passed from the RaftNode to the RaftServer via response_queue.
    Note that ResponseMessage must be passed to prevent a TimeoutError,
    although the timing of its delivery varies depending on the type of message.
    """

    pass


@dataclass
class WrongLeaderRespMessage(ResponseMessage):
    leader_id: int
    leader_addr: str


@dataclass
class JoinSuccessRespMessage(ResponseMessage):
    assigned_id: int
    raw_peers: bytes


@dataclass
class RemovedPeerSuccessMessage(ResponseMessage):
    pass


@dataclass
class ConfChangeRejectMessage(ResponseMessage):
    pass


@dataclass
class IdReservedRespMessage(ResponseMessage):
    leader_id: int
    reserved_id: int
    raw_peers: bytes


@dataclass
class RaftRespMessage(ResponseMessage):
    data: bytes
    rejected: bool = False


@dataclass
class RaftErrorRespMessage(ResponseMessage):
    data: Optional[bytes]


@dataclass
class ClusterBootstrapReadyRespMessage(ResponseMessage):
    pass


@dataclass
class SendMessageRespMessage(ResponseMessage):
    pass


@dataclass
class PeerRemovalSuccessRespMessage(ResponseMessage):
    pass


@dataclass
class MemberBootstrapReadyRespMessage(ResponseMessage):
    pass


@dataclass
class DebugNodeResponse(ResponseMessage):
    result: dict


@dataclass
class DebugEntriesResponse(ResponseMessage):
    result: dict


@dataclass
class VersionResponse(ResponseMessage):
    result: str


@dataclass
class GetPeersResponse(ResponseMessage):
    raw_peers: bytes
