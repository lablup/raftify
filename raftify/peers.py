import json
from typing import Optional

from rraft import RawNode

from .peer import Peer, PeerState
from .utils import SocketAddr


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

    def to_encodeable(self) -> "Peers":
        peers = Peers({})
        for node_id, peer in self.data.items():
            peers[node_id] = Peer(peer.addr, peer.state)
        return peers

    def get(self, node_id: int) -> Optional[Peer]:
        return self.data.get(node_id)

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

    def get_node_id_by_addr(self, addr: str | SocketAddr) -> Optional[int]:
        """ """
        if isinstance(addr, str):
            addr = SocketAddr.from_str(addr)

        for node_id, peer in self.data.items():
            if peer.addr == addr:
                return node_id

        return None

    def connect(self, id: int, addr: SocketAddr) -> None:
        """ """
        self.data[id] = Peer(addr, PeerState.Connected)
