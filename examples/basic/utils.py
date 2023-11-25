from pathlib import Path
from typing import Any, Iterable

import tomli
from aiohttp import web
from aiohttp.web import AbstractRouteDef

from raftify.config import RaftifyConfig
from raftify.peers import Peer, Peers
from raftify.utils import SocketAddr


def load_peer_candidates() -> list[SocketAddr]:
    path = Path(__file__).parent / "peer-candidates.toml"
    return [
        SocketAddr.from_str(addr)
        for addr in tomli.loads(path.read_text())["raft"]["peer-candidates"]
    ]


def load_peers() -> Peers:
    path = Path(__file__).parent / "config.toml"
    cfg = tomli.loads(path.read_text())["raft"]["peers"]

    return Peers(
        {
            int(entry["node_id"]): Peer(SocketAddr(entry["ip"], entry["port"]))
            for entry in cfg
        }
    )


def build_config():
    return RaftifyConfig(
        log_dir="./logs",
        auto_remove_node=False,
        compacted_log_dir="./logs",
        raft_config=RaftifyConfig.new_raft_config(
            {
                "election_tick": 10,
                "heartbeat_tick": 3,
            }
        ),
    )


class WebServer:
    def __init__(
        self, addr: str, routes: Iterable[AbstractRouteDef], state: dict[str, Any]
    ):
        self.app = web.Application()
        self.app.add_routes(routes)
        self.app["state"] = state
        self.host, self.port = addr.split(":")
        self.runner = None

    async def __aenter__(self):
        self.runner = web.AppRunner(self.app)
        await self.runner.setup()
        self.site = web.TCPSite(self.runner, self.host, self.port)
        await self.site.start()
        return self.runner

    async def __aexit__(self, exc_type, exc, tb):
        await self.runner.cleanup()
        await self.runner.shutdown()
