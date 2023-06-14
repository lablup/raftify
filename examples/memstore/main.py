import argparse
import asyncio
from contextlib import suppress
import logging
from threading import Lock
from typing import Optional
import os
import pickle

from aiohttp import web
from aiohttp.web import Application, RouteTableDef
from rraft import default_logger
from riteraft.fsm import FSM
from riteraft.mailbox import Mailbox
from riteraft.raft_facade import RaftClusterFacade


def setup_logger():
    # Set up rraft-py's slog log-level to Debug.
    os.environ["RUST_LOG"] = "debug"
    logging.basicConfig(level=logging.DEBUG)
    return default_logger()


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
            logging.info(f'Inserted: ({message.key}, "{message.value}")')
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
    mailbox: Mailbox = request.app["state"]["mailbox"]
    id, name = request.match_info["id"], request.match_info["name"]
    message = SetCommand(int(id), name)
    result = await mailbox.send(message.encode())
    return web.Response(text=str(result))


@routes.get("/leave")
async def leave(request: web.Request) -> web.Response:
    mailbox: Mailbox = request.app["state"]["mailbox"]
    await mailbox.leave()
    return web.Response(text="OK")


async def main() -> None:
    setup_logger()
    parser = argparse.ArgumentParser()
    parser.add_argument("--raft-addr", required=True)
    parser.add_argument("--peer-addr", default=None)
    parser.add_argument("--web-server", default=None)

    args = parser.parse_args()

    raft_addr = args.raft_addr
    peer_addr = args.peer_addr
    web_server_addr = args.web_server

    store = HashStore()
    raft_cluster = RaftClusterFacade(raft_addr, store, logger)
    mailbox = raft_cluster.mailbox()

    tasks = []
    if peer_addr:
        logger.info("Running in follower mode")
        tasks.append(raft_cluster.join(peer_addr))
    else:
        logger.info("Running in leader mode")
        tasks.append(raft_cluster.lead())

    runner = None
    if web_server_addr:
        app = Application()
        app.add_routes(routes)
        app["state"] = {
            "store": store,
            "mailbox": mailbox,
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
