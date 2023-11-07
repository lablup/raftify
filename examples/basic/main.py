import asyncio
import json
import pickle
from typing import cast

from aiohttp import web
from aiohttp.web import RouteTableDef

from examples.basic.logger import logger, slog
from examples.basic.utils import WebServer, build_config, load_peers
from raftify.cli import AbstractCLIContext
from raftify.deserializer import init_rraft_py_deserializer
from raftify.raft_facade import RaftFacade
from raftify.state_machine.hashstore import HashStore, SetCommand
from raftify.utils import SocketAddr

routes = RouteTableDef()


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


class RaftifyCLIContext(AbstractCLIContext):
    def __init__(self) -> None:
        super().__init__()
        init_rraft_py_deserializer()

    async def bootstrap_cluster(self, _args, options):
        web_server_addr = options.get("web_server")

        leader_node_id = 1
        initial_peers = load_peers()
        leader_addr = initial_peers[leader_node_id].addr

        store = HashStore()
        cfg = build_config()
        raft = RaftFacade(cfg, leader_addr, store, slog, logger, initial_peers)

        logger.info("Bootstrapping Raft Cluster...")
        raft.run_raft(node_id=1)

        async with WebServer(web_server_addr, routes, {"raft": raft}):
            await asyncio.gather(
                raft.wait_for_followers_join(),
                raft.wait_for_termination(),
            )

    async def bootstrap_follower(self, _args, options):
        raft_addr = SocketAddr.from_str(options["raft_addr"])
        web_server_addr = options["web_server"]

        initial_peers = load_peers()
        store = HashStore()
        cfg = build_config()
        raft = RaftFacade(cfg, raft_addr, store, slog, logger, initial_peers)

        logger.info("Participating in Raft Cluster...")
        node_id = initial_peers.get_node_id_by_addr(raft_addr)
        assert node_id is not None, "Member node_id is not found"

        raft.run_raft(node_id)
        await raft.send_member_bootstrap_ready_msg(node_id)

        async with WebServer(web_server_addr, routes, {"raft": raft}):
            await asyncio.gather(
                raft.wait_for_termination(),
            )

    async def add_member(self, _args, options):
        raft_addr = SocketAddr.from_str(options["raft_addr"])
        web_server_addr = options["web_server"]

        initial_peers = load_peers()
        peer_addrs = list(map(lambda peer: peer.addr, initial_peers))

        store = HashStore()
        cfg = build_config()
        raft = RaftFacade(cfg, raft_addr, store, slog, logger)

        request_id_resp = await raft.request_id(raft_addr, peer_addrs)
        raft.run_raft(request_id_resp.follower_id)
        logger.info("Running in follower mode...")
        await raft.join_cluster(request_id_resp)

        async with WebServer(web_server_addr, routes, {"raft": raft}):
            await asyncio.gather(
                raft.wait_for_termination(),
            )
