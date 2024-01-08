import argparse
import asyncio
import pickle
from contextlib import suppress
from pathlib import Path
from typing import Any, Iterable, Optional

import tomli
from aiohttp import web
from aiohttp.web import AbstractRouteDef, RouteTableDef
from raftify import (
    Config,
    Peers,
    Raft,
    RaftConfig,
    Logger,
    set_confchange_context_deserializer,
    set_confchangev2_context_deserializer,
    set_entry_context_deserializer,
    set_entry_data_deserializer,
    set_message_context_deserializer,
    set_snapshot_data_deserializer,
)


def load_peers() -> Peers:
    path = Path(__file__).parent / "cluster_config.toml"
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
    store: HashStore = request.app["state"]["store"]
    id = request.match_info["id"]
    return web.Response(text=store.get(id))


@routes.get("/leader")
async def leader(request: web.Request) -> web.Response:
    raft: Raft = request.app["state"]["raft"]
    leader_id = str(await raft.get_raft_node().get_leader_id())
    return web.Response(text=leader_id)


@routes.get("/size")
async def size(request: web.Request) -> web.Response:
    raft: Raft = request.app["state"]["raft"]
    size = str(await raft.get_raft_node().get_cluster_size())
    return web.Response(text=size)


@routes.get("/put/{id}/{value}")
async def put(request: web.Request) -> web.Response:
    raft: Raft = request.app["state"]["raft"]
    id, value = request.match_info["id"], request.match_info["value"]
    message = SetCommand(id, value)

    raft_node = raft.get_raft_node()
    raft_node.prepare_proposal(message.encode())
    await raft_node.propose()
    return web.Response(text="OK")


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


def pickle_deserialize(data: bytes) -> str | None:
    if data == b"":
        return None

    if pickle.PROTO in data:
        r = pickle.loads(data[data.index(pickle.PROTO) :])
        return r

    # Not pickle data
    return None


def register_custom_deserializer() -> None:
    """
    Initialize the custom deserializers.
    """

    set_confchange_context_deserializer(pickle_deserialize)
    set_confchangev2_context_deserializer(pickle_deserialize)
    set_entry_context_deserializer(pickle_deserialize)
    set_entry_data_deserializer(pickle_deserialize)
    set_message_context_deserializer(pickle_deserialize)
    set_snapshot_data_deserializer(pickle_deserialize)


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


async def wait_for_termination(raft: Raft):
    while True:
        if raft.is_finished():
            break
        await asyncio.sleep(1)


async def main():
    register_custom_deserializer()
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
    web_server_addr = args.web_server
    ignore_static_bootstrap = args.ignore_static_bootstrap

    peers = load_peers() if not ignore_static_bootstrap else None

    cfg = build_config()
    logger = Logger.default()
    store = HashStore()
    tasks = []

    if peer_addr:
        if not peers:
            join_ticket = await Raft.request_id(peer_addr)
            node_id = join_ticket.get_reserved_id()
        else:
            node_id = peers.get_node_id_by_addr(raft_addr)

        raft = Raft.build(node_id, raft_addr, store, cfg, logger, peers)
        await raft.run()

        if not peers:
            raft.prepare_join(join_ticket)
            await raft.join()
        else:
            leader_addr = peers.get(1)
            raft.prepare_member_bootstrap_ready(leader_addr, node_id)
            await raft.member_bootstrap_ready()
    else:
        raft = Raft.build(1, raft_addr, store, cfg, logger, peers)
        await raft.run()

    tasks.append(asyncio.create_task(wait_for_termination(raft)))

    async with WebServer(web_server_addr, routes, {"raft": raft, "store": store}):
        await asyncio.gather(*tasks)


if __name__ == "__main__":
    with suppress(KeyboardInterrupt):
        asyncio.run(main())
