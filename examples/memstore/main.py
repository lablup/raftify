import argparse
import asyncio
import logging
import os
import pickle
from contextlib import suppress
from pathlib import Path
from threading import Lock
from typing import Optional

import tomli
from aiohttp import web
from aiohttp.web import Application, RouteTableDef
from rraft import Logger, default_logger

from riteraft.fsm import FSM
from riteraft.raft_facade import RaftCluster
from riteraft.utils import SocketAddr


def setup_logger() -> Logger:
    # Set up rraft-py's slog log-level to Debug.
    # TODO: This method should be improved in rraft-py.
    os.environ["RUST_LOG"] = "debug"
    logging.basicConfig(level=logging.DEBUG)
    return default_logger()


def load_peer_candidates() -> list[SocketAddr]:
    path = Path(__file__).parent / "config.toml"
    return [
        SocketAddr.from_str(addr)
        for addr in tomli.loads(path.read_text())["raft"]["peer-candidates"]
    ]


logger = setup_logger()
routes = RouteTableDef()


class SetCommand:
    def __init__(self, key: int, value: str) -> None:
        self.key = key
        self.value = value

    def encode(self) -> bytes:
        return pickle.dumps(self.__dict__)

    @classmethod
    def decode(cls, packed: bytes) -> "SetCommand":
        unpacked = pickle.loads(packed)
        return cls(unpacked["key"], unpacked["value"])


class HashStore(FSM):
    def __init__(self):
        self._store = dict()
        self._lock = Lock()

    def get(self, key: int) -> Optional[str]:
        with self._lock:
            return self._store.get(key)

    async def apply(self, msg: bytes) -> bytes:
        with self._lock:
            message = SetCommand.decode(msg)
            self._store[message.key] = message.value
            logging.info(f'SetCommand inserted: ({message.key}, "{message.value}")')
            return pickle.dumps(message.value)

    async def snapshot(self) -> bytes:
        with self._lock:
            return pickle.dumps(self._store)

    async def restore(self, snapshot: bytes) -> None:
        with self._lock:
            self._store = pickle.loads(snapshot)


@routes.get("/get/{id}")
async def get(request: web.Request) -> web.Response:
    store: HashStore = request.app["state"]["store"]
    id = request.match_info["id"]
    return web.Response(text=store.get(int(id)))


@routes.get("/put/{id}/{name}")
async def put(request: web.Request) -> web.Response:
    cluster: RaftCluster = request.app["state"]["cluster"]
    id, name = request.match_info["id"], request.match_info["name"]
    message = SetCommand(int(id), name)
    result = await cluster.mailbox.send(message.encode())
    return web.Response(text=f'"{str(pickle.loads(result))}"')


@routes.get("/leave")
async def leave(request: web.Request) -> web.Response:
    cluster: RaftCluster = request.app["state"]["cluster"]

    await cluster.mailbox.leave(cluster.raft_node.get_id())
    return web.Response(
        text=f'Removed "node {cluster.raft_node.get_id()}" from the cluster successfully.'
    )


@routes.get("/remove/{id}")
async def remove(request: web.Request) -> web.Response:
    cluster: RaftCluster = request.app["state"]["cluster"]
    id = request.match_info["id"]

    await cluster.mailbox.leave(int(id))
    return web.Response(text=f'Removed "node {id}" from the cluster successfully.')


@routes.get("/peers")
async def peers(request: web.Request) -> web.Response:
    cluster: RaftCluster = request.app["state"]["cluster"]
    return web.Response(text=str(cluster.get_peers()))


async def main() -> None:
    setup_logger()
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--bootstrap", action=argparse.BooleanOptionalAction, default=None
    )
    parser.add_argument("--raft-addr", default=None)
    parser.add_argument("--web-server", default=None)

    args = parser.parse_args()

    bootstrap = args.bootstrap
    raft_addr = (
        SocketAddr.from_str(args.raft_addr) if args.raft_addr is not None else None
    )
    web_server_addr = args.web_server

    peer_addrs = load_peer_candidates()

    store = HashStore()

    tasks = []
    if bootstrap:
        assert raft_addr is None, "Cannot specify both --bootstrap and --raft-addr."

        cluster = RaftCluster(peer_addrs[0], store, logger)
        logger.info("Bootstrap a cluster")
        tasks.append(cluster.bootstrap_cluster())
    else:
        assert (
            raft_addr is not None
        ), "Follower node requires a --raft-addr option to join the cluster"

        logger.info("Running in follower mode")
        cluster = RaftCluster(raft_addr, store, logger)
        tasks.append(cluster.join_cluster(raft_addr, peer_addrs))

    runner = None
    if web_server_addr:
        app = Application()
        app.add_routes(routes)
        app["state"] = {
            "store": store,
            "cluster": cluster,
        }

        host, port = web_server_addr.split(":")
        runner = web.AppRunner(app)
        await runner.setup()
        site = web.TCPSite(runner, host, port)
        tasks.append(site.start())

    try:
        await asyncio.gather(*tasks)
    finally:
        if runner:
            await runner.cleanup()
            await runner.shutdown()


if __name__ == "__main__":
    with suppress(KeyboardInterrupt):
        asyncio.run(main())
