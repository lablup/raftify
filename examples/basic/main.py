import argparse
import asyncio
import json
import logging
import os
import pickle
from contextlib import suppress
from pathlib import Path
from threading import Lock
from typing import Optional

import colorlog
import tomli
from aiohttp import web
from aiohttp.web import Application, RouteTableDef
from rraft import Logger as Slog
from rraft import default_logger

from raftify.config import RaftifyConfig
from raftify.deserializer import init_rraft_py_deserializer
from raftify.fsm import FSM
from raftify.raft_cluster import RaftCluster
from raftify.raft_utils import leave_joint
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
    store: HashStore = request.app["state"]["store"]
    id = request.match_info["id"]
    return web.Response(text=store.get(id))


@routes.get("/all")
async def all(request: web.Request) -> web.Response:
    store: HashStore = request.app["state"]["store"]
    return web.Response(text=json.dumps(store.as_dict()))


@routes.get("/put/{id}/{value}")
async def put(request: web.Request) -> web.Response:
    cluster: RaftCluster = request.app["state"]["cluster"]
    id, value = request.match_info["id"], request.match_info["value"]
    message = SetCommand(id, value)
    result = await cluster.mailbox.send(message.encode())
    return web.Response(text=f'"{str(pickle.loads(result))}"')


@routes.get("/leave")
async def leave(request: web.Request) -> web.Response:
    cluster: RaftCluster = request.app["state"]["cluster"]
    id = cluster.raft_node.get_id()
    addr = cluster.get_peers()[id].addr
    await cluster.mailbox.leave(id, addr)
    return web.Response(text=f'Removed "node {id}" from the cluster successfully.')


@routes.get("/remove/{id}")
async def remove(request: web.Request) -> web.Response:
    cluster: RaftCluster = request.app["state"]["cluster"]
    id = int(request.match_info["id"])
    addr = cluster.get_peers()[id].addr
    await cluster.mailbox.leave(id, addr)
    return web.Response(text=f'Removed "node {id}" from the cluster successfully.')


@routes.get("/peers")
async def peers(request: web.Request) -> web.Response:
    cluster: RaftCluster = request.app["state"]["cluster"]
    return web.Response(text=str(cluster.get_peers()))


@routes.get("/leader")
async def leader(request: web.Request) -> web.Response:
    cluster: RaftCluster = request.app["state"]["cluster"]
    return web.Response(text=str(cluster.raft_node.get_leader_id()))


@routes.get("/progress")
async def show_progress(request: web.Request) -> web.Response:
    cluster: RaftCluster = request.app["state"]["cluster"]
    if not cluster.raft_node.is_leader():
        return web.Response(text="Not leader.")

    progress_tracker = cluster.raft_node.raw_node.get_raft().prs().collect()
    res = [str(pr_tracker.progress()) for pr_tracker in progress_tracker]
    return web.Response(text=str(res))


@routes.get("/merge/{id}/{addr}")
async def merge_cluster(request: web.Request) -> web.Response:
    cluster: RaftCluster = request.app["state"]["cluster"]
    target_node_id = int(request.match_info["id"])
    target_addr = request.match_info["addr"]

    peer_addrs = load_peer_candidates()
    await cluster.merge_cluster(
        target_node_id, SocketAddr.from_str(target_addr), peer_addrs
    )

    return web.Response(text="Merged cluster successfully.")


@routes.get("/transfer/{id}")
async def transfer_leader(request: web.Request) -> web.Response:
    cluster: RaftCluster = request.app["state"]["cluster"]
    target_node_id = int(request.match_info["id"])

    cluster.transfer_leader(target_node_id)
    return web.Response(text="Leader transferred successfully.")


@routes.get("/snapshot")
async def snapshot(request: web.Request) -> web.Response:
    cluster: RaftCluster = request.app["state"]["cluster"]

    await cluster.create_snapshot()
    return web.Response(text="Created snapshot successfully.")


@routes.get("/entries")
async def entries(request: web.Request) -> web.Response:
    cluster: RaftCluster = request.app["state"]["cluster"]

    all_entries = cluster.raft_node.raw_node.get_raft().get_raft_log().all_entries()
    res = f"[ {', '.join(list(map(lambda e: str(e), all_entries)))} ]"

    return web.Response(text=res)


@routes.get("/unstable")
async def unstable(request: web.Request) -> web.Response:
    cluster: RaftCluster = request.app["state"]["cluster"]
    return web.Response(
        text=str(cluster.raft_node.raw_node.get_raft().get_raft_log().unstable())
    )


@routes.get("/zero")
async def zero(request: web.Request) -> web.Response:
    cluster: RaftCluster = request.app["state"]["cluster"]
    await leave_joint(cluster.raft_node)

    return web.Response(text=str(""))


@routes.get("/debug")
async def debug(request: web.Request) -> web.Response:
    cluster: RaftCluster = request.app["state"]["cluster"]
    return web.Response(text=cluster.get_debug_info())


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

    peer_addrs = load_peer_candidates()

    store = HashStore()

    target_addr = peer_addrs[0] if bootstrap and not raft_addr else raft_addr

    cfg = RaftifyConfig(
        log_dir="./",
        raft_config=RaftifyConfig.new_raft_config(
            {
                "election_tick": 10,
                "heartbeat_tick": 3,
            }
        ),
    )

    cluster = RaftCluster(cfg, target_addr, store, slog, logger)
    tasks = []

    if bootstrap:
        logger.info("Bootstrap a Raft Cluster")
        node_id = 1
        cluster.run_raft(node_id)
    else:
        assert (
            raft_addr is not None
        ), "Follower node requires a --raft-addr option to join the cluster"

        request_id_resp = await cluster.request_id(raft_addr, peer_addrs)
        cluster.run_raft(request_id_resp.follower_id)
        logger.info("Running in follower mode")
        await cluster.join_cluster(request_id_resp)

    tasks.append(cluster.wait_for_termination())

    app_runner = None
    if web_server_addr:
        app = Application()
        app.add_routes(routes)
        app["state"] = {
            "store": store,
            "cluster": cluster,
        }

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
