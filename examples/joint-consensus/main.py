import argparse
import asyncio
import json
import logging
import os
import pickle
from contextlib import suppress
from pathlib import Path
from threading import Lock
from typing import Optional, cast

import colorlog
import tomli
from aiohttp import web
from aiohttp.web import Application, RouteTableDef
from rraft import Logger as Slog
from rraft import default_logger

from raftify.config import RaftifyConfig
from raftify.deserializer import init_rraft_py_deserializer
from raftify.fsm import FSM
from raftify.peers import Peer, Peers
from raftify.raft_facade import RaftFacade
from raftify.utils import SocketAddr


def setup_slog() -> Slog:
    # TODO: This method should be implemented in rraft-py.
    # Set up rraft-py's slog log-level to Debug.
    os.environ["RUST_LOG"] = "debug"
    return default_logger()


def setup_logger() -> logging.Logger:
    log_format = "%(asctime)s - " "%(log_color)s%(levelname)-8s - %(message)s%(reset)s"

    log_colors_config = {
        "DEBUG": "cyan",
        "INFO": "green",
        "WARNING": "yellow",
        "ERROR": "red",
        "CRITICAL": "red",
        "asctime": "grey",
    }

    colorlog.basicConfig(
        level=logging.DEBUG, format=log_format, log_colors=log_colors_config
    )
    return logging.getLogger()


def load_peers() -> Peers:
    path = Path(__file__).parent / "config.toml"
    cfg = tomli.loads(path.read_text())["raft"]["peers"]

    return Peers(
        {
            int(entry["node_id"]): Peer(addr=SocketAddr(entry["ip"], entry["port"]))
            for entry in cfg
        }
    )


slog = setup_slog()
logger = setup_logger()
routes = RouteTableDef()


class SetCommand:
    def __init__(self, key: str, value: str) -> None:
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

    def get(self, key: str) -> Optional[str]:
        with self._lock:
            return self._store.get(key)

    def as_dict(self) -> dict:
        return self._store

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
    raft_facade: RaftFacade = request.app["state"]["raft"]
    store = cast(HashStore, raft_facade.store)

    id = request.match_info["id"]
    return web.Response(text=store.get(id))


@routes.get("/all")
async def all(request: web.Request) -> web.Response:
    raft_facade: RaftFacade = request.app["state"]["raft"]
    store = cast(HashStore, raft_facade.store)

    return web.Response(text=json.dumps(store.as_dict()))


@routes.get("/put/{id}/{value}")
async def put(request: web.Request) -> web.Response:
    raft_facade: RaftFacade = request.app["state"]["raft"]
    id, value = request.match_info["id"], request.match_info["value"]
    message = SetCommand(id, value)
    result = await raft_facade.mailbox.send(message.encode())
    return web.Response(text=f'"{str(pickle.loads(result))}"')


@routes.get("/leave")
async def leave(request: web.Request) -> web.Response:
    raft_facade: RaftFacade = request.app["state"]["raft"]
    id = raft_facade.raft_node.get_id()
    addr = raft_facade.peers[id].addr
    await raft_facade.mailbox.leave(id, addr)
    return web.Response(text=f'Removed "node {id}" from the cluster successfully.')


@routes.get("/peers")
async def peers(request: web.Request) -> web.Response:
    raft_facade: RaftFacade = request.app["state"]["raft"]
    return web.Response(text=str(raft_facade.peers))


async def main() -> None:
    init_rraft_py_deserializer()

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
    peers = load_peers()
    store = HashStore()
    target_addr = peers[1].addr if bootstrap and not raft_addr else raft_addr

    cfg = RaftifyConfig(
        log_dir="./logs",
        raft_config=RaftifyConfig.new_raft_config(
            {
                "election_tick": 10,
                "heartbeat_tick": 3,
            }
        ),
    )

    raft = RaftFacade(cfg, target_addr, store, slog, logger, peers)
    tasks = []

    if bootstrap:
        logger.info("Bootstrap a Raft Cluster")
        node_id = 1
        raft.run_raft(node_id)
        tasks.append(raft.wait_for_followers_join())
        tasks.append(raft.wait_for_termination())
    else:
        assert (
            raft_addr is not None
        ), "Follower node requires a --raft-addr option to join the cluster"

        logger.info("Running in follower mode")
        node_id = peers.get_node_id_by_addr(raft_addr)
        assert node_id is not None, "Member Node id is not found"

        raft.run_raft(node_id)
        await raft.send_member_bootstrap_ready_msg(node_id)
        tasks.append(raft.wait_for_termination())

    app_runner = None
    if web_server_addr:
        app = Application()
        app.add_routes(routes)
        app["state"] = {"raft": raft}

        host, port = web_server_addr.split(":")
        app_runner = web.AppRunner(app)
        await app_runner.setup()
        web_server = web.TCPSite(app_runner, host, port)
        tasks.append(web_server.start())

    try:
        await asyncio.gather(*tasks)
    finally:
        if app_runner:
            await app_runner.cleanup()
            await app_runner.shutdown()


if __name__ == "__main__":
    with suppress(KeyboardInterrupt):
        asyncio.run(main())
