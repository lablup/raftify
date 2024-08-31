from typing import Any, Iterable

from aiohttp import web
from aiohttp.web import AbstractRouteDef, RouteTableDef
from raftify import Raft

from .state_machine import HashStore, SetCommand


class WebServer:
    def __init__(
        self, addr: str, routes: Iterable[AbstractRouteDef], state: dict[str, Any]
    ):
        self.app = web.Application()
        self.app.add_routes(routes)
        self.app["state"] = state
        self.ip, self.port = addr.split(":")
        self.runner = None

    async def __aenter__(self):
        self.runner = web.AppRunner(self.app)
        await self.runner.setup()
        self.site = web.TCPSite(self.runner, self.ip, self.port)
        await self.site.start()
        return self.runner

    async def __aexit__(self, exc_type, exc, tb):
        await self.runner.cleanup()
        await self.runner.shutdown()


routes = RouteTableDef()
"""
APIs of the web servers to interact with the RaftServers.
"""


@routes.get("/store/{id}")
async def get(request: web.Request) -> web.Response:
    store: HashStore = request.app["state"]["store"]
    id = request.match_info["id"]
    return web.Response(text=store.get(id))


@routes.put("/store/{id}/{value}")
async def put(request: web.Request) -> web.Response:
    raft: Raft = request.app["state"]["raft"]
    id, value = request.match_info["id"], request.match_info["value"]
    message = SetCommand(id, value)

    raft_node = raft.get_raft_node()
    await raft_node.propose(message.encode())
    return web.Response(text="OK")


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
