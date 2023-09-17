import json
from collections import UserDict
from typing import ItemsView, Optional

from raftify.raft_client import RaftClient


class Peers(UserDict):
    # the peer client could be optional, because an id can be reserved and later populated
    data: dict[int, Optional[RaftClient]]

    def __getitem__(self, key: int | str) -> RaftClient:
        return super().__getitem__(key)

    def __repr__(self) -> str:
        return json.dumps(
            {
                str(key): value.to_dict() if value is not None else None
                for key, value in self.data.items()
            }
        )

    def items(self) -> ItemsView[int, Optional[RaftClient]]:
        return self.data.items()

    def add_peer(self, peer_id: int, raft_client: RaftClient):
        self.data[peer_id] = raft_client

    def remove_peer(self, peer_id: int):
        if peer_id in self.data:
            del self.data[peer_id]

    def get_peer(self, peer_id: int) -> Optional[RaftClient]:
        return self.data.get(peer_id)

    def peer_addrs(self) -> dict[int, str]:
        return {k: str(v.addr) for k, v in self.data.items() if v is not None}
