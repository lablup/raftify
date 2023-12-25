import argparse
import asyncio
from contextlib import suppress
import pickle
from typing import Optional
from raftify import (
    Peers,
    Raft,
    Config,
    RaftConfig,
    # AbstractLogEntry,
    # AbstractStateMachine,
)
import tomli
from pathlib import Path
from typing import Any, Iterable

from aiohttp import web
from aiohttp.web import RouteTableDef, AbstractRouteDef


def load_peers() -> Peers:
    path = Path(__file__).parent / "config.toml"
    cfg = tomli.loads(path.read_text())["raft"]["peers"]

    return Peers(
        {int(entry["node_id"]): f"{entry['host']}:{entry['port']}" for entry in cfg}
    )


def build_config() -> Config:
    raft_cfg = RaftConfig()
    cfg = Config(
        raft_cfg,
        log_dir="./logs",
        compacted_log_dir="./logs",
    )

    return cfg


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


routes = RouteTableDef()
"""
APIs of the web servers to interact with the RaftServers.
"""


@routes.get("/get/{id}")
async def get(request: web.Request) -> web.Response:
    store = request.app["state"]["store"]
    id = request.match_info["id"]
    return web.Response(text=store.get(id))


@routes.get("/put/{id}/{value}")
async def put(request: web.Request) -> web.Response:
    raft = request.app["state"]["raft"]
    id, value = request.match_info["id"], request.match_info["value"]
    message = SetCommand(id, value)

    result = await raft.propose(message.encode())
    return web.Response(text=f'"{str(result)}"')


# class SetCommand(AbstractLogEntry):
class SetCommand:
    """
    Represent simple key-value command.
    Use pickle to serialize the data.
    """

    def __init__(self, key: str, value: str) -> None:
        self.key = key
        self.value = value

    def encode(self) -> bytes:
        return pickle.dumps(self.__dict__)

    @classmethod
    def decode(cls, packed: bytes) -> "SetCommand":
        unpacked = pickle.loads(packed)
        return cls(unpacked["key"], unpacked["value"])


# class HashStore(AbstractStateMachine):


class HashStore:
    """
    A simple key-value store that stores data in memory.
    Use pickle to serialize the data.
    """

    def __init__(self):
        self._store = dict()

    def get(self, key: str) -> Optional[str]:
        return self._store.get(key)

    def as_dict(self) -> dict:
        return self._store

    def apply(self, msg: bytes) -> bytes:
        message = SetCommand.decode(msg)
        self._store[message.key] = message.value
        return msg

    def snapshot(self) -> bytes:
        return pickle.dumps(self._store)

    def restore(self, snapshot: bytes) -> None:
        self._store = pickle.loads(snapshot)


async def main():
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--ignore-static-bootstrap", action=argparse.BooleanOptionalAction, default=None
    )
    parser.add_argument("--raft-addr", default=None)
    parser.add_argument("--peer-addr", default=None)
    parser.add_argument("--web-server", default=None)
    args = parser.parse_args()

    raft_addr = args.raft_addr
    peer_addr = args.peer_addr
    # web_server = args.web_server
    # ignore_static_bootstrap = args.ignore_static_bootstrap

    cfg = build_config()
    store = HashStore()
    tasks = []

    join_ticket = await Raft.request_id(peer_addr) if peer_addr else None

    raft = Raft.build(raft_addr, store, cfg, join_ticket)
    tasks.append(asyncio.create_task(raft.run()))

    await asyncio.gather(tasks)


if __name__ == "__main__":
    with suppress(KeyboardInterrupt):
        asyncio.run(main())
