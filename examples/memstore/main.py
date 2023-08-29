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

import tomli
from aiohttp import web
from aiohttp.web import Application, RouteTableDef
from rraft import Logger as Slog
from rraft import default_logger

from raftify.config import RaftConfig
from raftify.deserializer import init_deserializer
from raftify.fsm import FSM
from raftify.raft_facade import FollowerRole, RaftCluster, RaftNodeRole
from raftify.utils import SocketAddr

init_deserializer()

RaftCluster.set_cluster_config(
    RaftConfig(
        log_dir="./",
        use_log_compaction=False,
        config=RaftConfig.new_raft_config(
            {
                "election_tick": 10,
                "heartbeat_tick": 3,
            }
        ),
    )
)


def setup_slog() -> Slog:
    # TODO: This method should be implemented in rraft-py.
    # Set up rraft-py's slog log-level to Debug.
    os.environ["RUST_LOG"] = "debug"
    return default_logger()


def setup_logger() -> logging.Logger:
    logging.basicConfig(level=logging.DEBUG)
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
    return web.Response(text=store.get(int(id)))


@routes.get("/all")
async def all(request: web.Request) -> web.Response:
    store: HashStore = request.app["state"]["store"]
    return web.Response(text=json.dumps(store.as_dict()))


@routes.get("/put/{id}/{value}")
async def put(request: web.Request) -> web.Response:
    cluster: RaftCluster = request.app["state"]["cluster"]
    id, value = request.match_info["id"], request.match_info["value"]
    message = SetCommand(int(id), value)
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


@routes.get("/leader")
async def leader(request: web.Request) -> web.Response:
    cluster: RaftCluster = request.app["state"]["cluster"]
    return web.Response(text=str(cluster.raft_node.get_leader_id()))


async def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--bootstrap", action=argparse.BooleanOptionalAction, default=None
    )
    parser.add_argument("--raft-addr", default=None)
    parser.add_argument("--web-server", default=None)
    parser.add_argument(
        "--non-voter", action=argparse.BooleanOptionalAction, default=None
    )

    args = parser.parse_args()

    bootstrap = args.bootstrap
    raft_addr = (
        SocketAddr.from_str(args.raft_addr) if args.raft_addr is not None else None
    )
    web_server_addr = args.web_server
    follower_role = FollowerRole.Learner if args.non_voter else FollowerRole.Voter

    peer_addrs = load_peer_candidates()

    store = HashStore()

    tasks = []
    target_addr = peer_addrs[0] if bootstrap else raft_addr
    cluster = RaftCluster(target_addr, store, slog, logger)

    if bootstrap:
        assert raft_addr is None, "Cannot specify both --bootstrap and --raft-addr."
        logger.info("Bootstrap a Raft Cluster")
        cluster.build_raft(RaftNodeRole.LEADER)
        tasks.append(cluster.bootstrap_cluster())
    else:
        assert (
            raft_addr is not None
        ), "Follower node requires a --raft-addr option to join the cluster"

        request_id_response = await cluster.request_id(raft_addr, peer_addrs)

        logger.info("Running in follower mode")
        cluster.build_raft(RaftNodeRole.Follower, request_id_response.follower_id)
        tasks.append(
            cluster.join_cluster(
                request_id_response=request_id_response, role=follower_role
            )
        )

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
