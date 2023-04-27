import argparse
import asyncio
from contextlib import suppress
import logging
from threading import Lock
from typing import Optional
import os
import msgpack

from aiohttp import web
from aiohttp.web import Application, RouteTableDef
from rraft import default_logger
from riteraft.mailbox import Mailbox

from riteraft.raft import Raft


def setup_logger():
    # Set up rraft-py's slog log-level to Debug.
    os.environ["RUST_LOG"] = "debug"
    logging.basicConfig(level=logging.DEBUG)
    return default_logger()


logger = setup_logger()
routes = RouteTableDef()


class InsertMessage:
    def __init__(self, key: int, value: str) -> None:
        self.key = key
        self.value = value

    def to_msgpack(self) -> bytes:
        return msgpack.packb(self.__dict__)

    @classmethod
    def from_msgpack(cls, packed: bytes) -> "InsertMessage":
        unpacked = msgpack.unpackb(packed)
        return cls(unpacked["key"], unpacked["value"])


class Options:
    def __init__(
        self,
        raft_addr: str,
        peer_addr: Optional[str] = None,
        web_server: Optional[str] = None,
    ):
        self.raft_addr = raft_addr
        self.peer_addr = peer_addr
        self.web_server = web_server


class HashStore:
    def __init__(self):
        self._store = dict()
        self._lock = Lock()

    def get(self, key: int) -> Optional[str]:
        with self._lock:
            return self._store.get(key)

    async def apply(self, msg: bytes) -> bytes:
        with self._lock:
            message = InsertMessage.from_msgpack(msg)
            self._store[message.key] = message.value
            logging.info(f'Inserted: ({message.key}, "{message.value}")')
            return msgpack.packb(message.value)

    async def snapshot(self) -> bytes:
        with self._lock:
            return msgpack.packb(self._store)

    async def restore(self, snapshot: bytes) -> None:
        with self._lock:
            self._store = msgpack.unpackb(snapshot)


@routes.get("/get/{id}")
async def get(request: web.Request) -> web.Response:
    store: HashStore = request.app["state"]["store"]
    id = request.match_info["id"]
    return web.Response(text=store.get(int(id)))


@routes.get("/put/{id}/{name}")
async def put(request: web.Request) -> web.Response:
    mailbox: Mailbox = request.app["state"]["mailbox"]
    id, name = request.match_info["id"], request.match_info["name"]
    message = InsertMessage(int(id), name)
    result = await mailbox.send(message.to_msgpack())
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

    options = Options(
        raft_addr=args.raft_addr,
        peer_addr=args.peer_addr,
        web_server=args.web_server,
    )

    store = HashStore()
    raft = Raft(options.raft_addr, store, logger)
    mailbox = raft.mailbox()

    tasks = []
    if options.peer_addr:
        logger.info("Running in follower mode")
        tasks.append(raft.join(options.peer_addr))
    else:
        logger.info("Running in leader mode")
        tasks.append(raft.lead())

    runner = None
    if options.web_server:
        app = Application()
        app.add_routes(routes)
        app["state"] = {
            "store": store,
            "mailbox": mailbox,
        }

        host, port = options.web_server.split(":")
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
