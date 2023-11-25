import json
import pickle
from typing import cast

from aiohttp import web
from aiohttp.web import RouteTableDef

from raftify.log_entry.set_command import SetCommand
from raftify.raft_facade import RaftFacade
from raftify.state_machine.hashstore import HashStore

routes = RouteTableDef()
"""
APIs of the web servers to interact with the RaftServers.
"""


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
    await raft_facade.mailbox.leave(id)
    return web.Response(text=f'Removed "node {id}" from the cluster successfully.')


@routes.get("/remove/{id}")
async def remove(request: web.Request) -> web.Response:
    raft_facade: RaftFacade = request.app["state"]["raft"]
    id = int(request.match_info["id"])
    await raft_facade.mailbox.leave(id)
    return web.Response(text=f'Removed "node {id}" from the cluster successfully.')


@routes.get("/peers")
async def peers(request: web.Request) -> web.Response:
    raft_facade: RaftFacade = request.app["state"]["raft"]
    return web.Response(text=str(raft_facade.peers))


@routes.get("/leader")
async def leader(request: web.Request) -> web.Response:
    raft_facade: RaftFacade = request.app["state"]["raft"]
    return web.Response(text=str(raft_facade.raft_node.get_leader_id()))


@routes.get("/transfer/{id}")
async def transfer_leader(request: web.Request) -> web.Response:
    raft_facade: RaftFacade = request.app["state"]["raft"]
    target_node_id = int(request.match_info["id"])

    raft_facade.raft_node.transfer_leader(target_node_id)
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
