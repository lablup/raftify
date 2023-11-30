from enum import StrEnum
from typing import Optional

from .logger import AbstractRaftifyLogger
from .raft_client import RaftClient
from .utils import SocketAddr


class PeerState(StrEnum):
    """
    Represent peer's state.
    Note that the PeerState may not be accurate during the time ConfChange is reflected.
    """

    Preparing = "Preparing"
    Connected = "Connected"
    Disconnected = "Disconnected"
    Disconnecting = "Disconnecting"
    Quitted = "Quitted"


class Peer:
    """
    Represents the socket address, client object, and state of each node at the network layer.
    """

    def __init__(
        self,
        addr: str | SocketAddr,
        state: PeerState = PeerState.Preparing,
        *,
        logger: Optional[AbstractRaftifyLogger] = None,
    ):
        if isinstance(addr, str):
            addr = SocketAddr.from_str(addr)

        self.addr = addr
        self.client = RaftClient(addr, logger=logger)
        self.state = state

    def to_dict(self) -> dict:
        return {
            "client": self.client.to_dict(),
            "addr": str(self.addr),
            "state": self.state,
        }
