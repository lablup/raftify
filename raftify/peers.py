import json
import pickle
from enum import StrEnum
from typing import Optional

from rraft import RawNode

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


class Peers:
    def __init__(self, peers: dict[int, Peer]) -> None:
        self.data = peers

    def __repr__(self) -> str:
        return json.dumps(
            {str(node_id): peer.to_dict() for node_id, peer in self.data.items()}
        )

    def __getitem__(self, node_id: int) -> Peer:
        assert node_id in self.data
        return self.data[node_id]

    def __setitem__(self, key: int, value: Peer):
        self.data[key] = value

    def __len__(self) -> int:
        return len(self.data)

    def __iter__(self):
        return iter(self.data.keys())

    def keys(self):
        return self.data.keys()

    def values(self):
        return self.data.values()

    def items(self):
        return self.data.items()

    def encode(self) -> bytes:
        peers = Peers({})
        for node_id, peer in self.data.items():
            peers[node_id] = Peer(peer.addr, peer.state)
        return pickle.dumps(peers)

    def get(self, node_id: int) -> Optional[Peer]:
        return self.data.get(node_id)

    @staticmethod
    def decode(bytes: bytes) -> "Peers":
        return pickle.loads(bytes)

    def reserve_peer(self, raw_node: RawNode, addr: SocketAddr) -> int:
        """ """
        prev_conns = [
            node_id
            for node_id, peer in self.data.items()
            if peer.addr == addr and peer.state == PeerState.Disconnected
        ]

        if len(prev_conns) > 0:
            next_id = prev_conns[0]
        else:
            next_id = max(self.data.keys()) if any(self.data) else 1
            next_id = max(next_id + 1, raw_node.get_raft().get_id())

            # if assigned id is ourself, return next one
            if next_id == raw_node.get_raft().get_id():
                next_id += 1

        self.data[next_id] = Peer(addr)
        return next_id

    def ready_peer(self, addr: SocketAddr) -> None:
        """ """
        for peer in self.data.values():
            if peer.addr == addr:
                peer.state = PeerState.Connected
                return

    def get_node_id_by_addr(self, addr: SocketAddr) -> Optional[int]:
        """ """
        for node_id, peer in self.data.items():
            if peer.addr == addr:
                return node_id

        return None

    def connect(self, id: int, addr: SocketAddr) -> None:
        """ """
        if id not in self.data:
            self.data[id] = Peer(addr, PeerState.Connected)
        else:
            self.data[id].addr = addr
            self.data[id].state = PeerState.Connected
            self.data[id].client = RaftClient(addr)
