from __future__ import annotations

from dataclasses import dataclass
from typing import TYPE_CHECKING, Any

if TYPE_CHECKING:
    from .peers import Peers

from .log_entry.abc import AbstractLogEntry
from .response_message import JoinSuccessRespMessage, PeerRemovalSuccessRespMessage


@dataclass
class ProposeResponse:
    msg: AbstractLogEntry
    rejected: bool


@dataclass
class ChangeConfigResponse:
    result: str
    data: JoinSuccessRespMessage | PeerRemovalSuccessRespMessage


@dataclass
class IdRequestResponse:
    result: str
    leader_id: int
    leader_addr: str
    reserved_id: int
    peers: Peers


@dataclass
class DebugNodeResponse:
    result: dict[str, Any]


@dataclass
class DebugEntriesResponse:
    result: dict[str, Any]


@dataclass
class VersionResponse:
    result: str


@dataclass
class GetPeersResponse:
    peers: Peers
