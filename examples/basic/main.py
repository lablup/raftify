import asyncio
import json
import logging
import os
import pickle
from pathlib import Path
from threading import Lock
from typing import Optional, cast
import argparse

import colorlog
import tomli
from aiohttp import web
from aiohttp.web import Application, RouteTableDef
from rraft import Logger as Slog
from rraft import default_logger

from raftify.cli import RaftifyContext
from raftify.config import RaftifyConfig
from raftify.deserializer import init_rraft_py_deserializer
from raftify.fsm import FSM
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


def load_peer_candidates() -> list[SocketAddr]:
    path = Path(__file__).parent / "config.toml"
    return [
        SocketAddr.from_str(addr)
        for addr in tomli.loads(path.read_text())["raft"]["peer-candidates"]
    ]


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


@routes.get("/remove/{id}")
async def remove(request: web.Request) -> web.Response:
    cluster: RaftFacade = request.app["state"]["cluster"]
    id = int(request.match_info["id"])
    addr = cluster.peers[id].addr
    await cluster.mailbox.leave(id, addr)
    return web.Response(text=f'Removed "node {id}" from the cluster successfully.')


@routes.get("/peers")
async def peers(request: web.Request) -> web.Response:
    raft_facade: RaftFacade = request.app["state"]["raft"]
    return web.Response(text=str(raft_facade.peers))


@routes.get("/leader")
async def leader(request: web.Request) -> web.Response:
    raft_facade: RaftFacade = request.app["state"]["raft"]
    return web.Response(text=str(raft_facade.raft_node.get_leader_id()))


@routes.get("/merge/{id}/{addr}")
async def merge_cluster(request: web.Request) -> web.Response:
    raft_facade: RaftFacade = request.app["state"]["raft"]
    target_node_id = int(request.match_info["id"])
    target_addr = request.match_info["addr"]

    peer_addrs = load_peer_candidates()
    await raft_facade.merge_cluster(
        target_node_id, SocketAddr.from_str(target_addr), peer_addrs
    )

    return web.Response(text="Merged cluster successfully.")


@routes.get("/transfer/{id}")
async def transfer_leader(request: web.Request) -> web.Response:
    raft_facade: RaftFacade = request.app["state"]["raft"]
    target_node_id = int(request.match_info["id"])

    raft_facade.transfer_leader(target_node_id)
    return web.Response(text="Leader transferred successfully.")


@routes.get("/snapshot")
async def snapshot(request: web.Request) -> web.Response:
    raft_facade: RaftFacade = request.app["state"]["raft"]

    await raft_facade.create_snapshot()
    return web.Response(text="Created snapshot successfully.")


@routes.get("/unstable")
async def unstable(request: web.Request) -> web.Response:
    raft_facade: RaftFacade = request.app["state"]["raft"]
    return web.Response(
        text=str(raft_facade.raft_node.raw_node.get_raft().get_raft_log().unstable())
    )


class Impl(RaftifyContext):
    def __init__(self) -> None:
        super().__init__()
        init_rraft_py_deserializer()

    async def bootstrap_cluster(self, args):
        web_server_addr = args[0]

        peer_addrs = load_peer_candidates()

        store = HashStore()

        target_addr = peer_addrs[0]

        cfg = RaftifyConfig(
            log_dir="./logs",
            raft_config=RaftifyConfig.new_raft_config(
                {
                    "election_tick": 10,
                    "heartbeat_tick": 3,
                }
            ),
        )

        raft = RaftFacade(cfg, target_addr, store, slog, logger)
        tasks = []

        logger.info("Bootstrap a Raft Cluster")
        node_id = 1
        raft.run_raft(node_id)

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

    async def bootstrap_member(self, args):
        raise NotImplementedError

    async def add_member(self, args) -> bytes:
        parser = argparse.ArgumentParser()
        # TODO: How can we handle this??
        parser.add_argument('addr', help='First option')
        parser.add_argument('--raft-addr', help='First option')
        parser.add_argument('--web-server', help='Second option')
        args = parser.parse_args()

        raft_addr = (
            SocketAddr.from_str(args.raft_addr) if args.raft_addr is not None else None
        )
        web_server_addr = args.web_server

        peer_addrs = load_peer_candidates()

        store = HashStore()

        target_addr = raft_addr

        cfg = RaftifyConfig(
            log_dir="./logs",
            raft_config=RaftifyConfig.new_raft_config(
                {
                    "election_tick": 10,
                    "heartbeat_tick": 3,
                }
            ),
        )

        raft = RaftFacade(cfg, target_addr, store, slog, logger)
        tasks = []

        request_id_resp = await raft.request_id(raft_addr, peer_addrs)
        raft.run_raft(request_id_resp.follower_id)
        logger.info("Running in follower mode")
        await raft.join_cluster(request_id_resp)

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
            pass
