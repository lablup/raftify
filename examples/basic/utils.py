from pathlib import Path

import tomli
from aiohttp import web
from aiohttp.web import Application

from raftify.config import RaftifyConfig
from raftify.peers import Peer, Peers
from raftify.raft_facade import RaftFacade
from raftify.utils import SocketAddr


def load_peers() -> Peers:
    path = Path(__file__).parent / "config.toml"
    cfg = tomli.loads(path.read_text())["raft"]["peers"]

    return Peers(
        {
            int(entry["node_id"]): Peer(SocketAddr(entry["ip"], entry["port"]))
            for entry in cfg
        }
    )


def build_raftify_cfg():
    return RaftifyConfig(
        log_dir="./logs",
        raft_config=RaftifyConfig.new_raft_config(
            {
                "election_tick": 10,
                "heartbeat_tick": 3,
            }
        ),
    )


async def run_webserver(routes, web_server_addr: str, raft: RaftFacade):
    app = Application()
    app.add_routes(routes)
    app["state"] = {"raft": raft}

    host, port = web_server_addr.split(":")
    app_runner = web.AppRunner(app)

    try:
        await app_runner.setup()
        web_server = web.TCPSite(app_runner, host, port)
        return web_server.start()
    finally:
        await app_runner.cleanup()
        await app_runner.shutdown()
