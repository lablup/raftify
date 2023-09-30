from dataclasses import dataclass
import json
from collections import UserDict
from typing import ItemsView, Optional

from raftify.raft_client import RaftClient
from raftify.utils import SocketAddr


@dataclass
class Peer:
    addr: SocketAddr
    client: Optional[RaftClient] = None
    ready: bool = False

    def to_dict(self) -> dict:
        return {
            "client": self.client.to_dict() if self.client is not None else None,
            "addr": str(self.addr),
            "ready": self.ready,
        }


class Peers(UserDict):
    # the peer client could be optional, because an id can be reserved and later populated
    data: dict[int, Peer]

    def __getitem__(self, key: int | str) -> Peer:
        return super().__getitem__(key)

    def __repr__(self) -> str:
        return json.dumps(
            {
                str(key): value.to_dict() if value is not None else None
                for key, value in self.data.items()
            }
        )

    def items(self) -> ItemsView[int, Peer]:
        return self.data.items()

    def peer_addrs(self) -> dict[int, str]:
        return {k: str(v.addr) for k, v in self.data.items() if v is not None}

    # def reserve_next_peer_id(self, addr: str) -> int:
    #     """
    #     Reserve a slot to insert node on next node addition commit.
    #     """
    #     prev_conns = [
    #         id for id, peer in self.items() if peer and addr == str(peer.addr)
    #     ]

    #     if len(prev_conns) > 0:
    #         next_id = prev_conns[0]
    #     else:
    #         next_id = max(self.data.keys()) if any(self.data) else 1
    #         next_id = max(next_id + 1, self.get_id())

    #         # if assigned id is ourself, return next one
    #         if next_id == self.get_id():
    #             next_id += 1

    #     # self.logger.info(f"Reserved peer id {next_id}.")
    #     self.data[next_id] = Peer(None, SocketAddr.from_str(addr))
    #     return next_id
