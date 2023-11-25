import asyncio
import json
import multiprocessing
import os
import pickle
import signal
import sys
from contextlib import asynccontextmanager as actxmgr
from typing import AsyncIterator

import aiotools
from aiohttp import web
from aiohttp.web import Application, RouteTableDef
from aiotools import process_index
from harness.constant import CLUSTER_INFO_PATH, RAFT_ADDRS, WEB_SERVER_ADDRS
from harness.logger import logger, slog
from utils import read_cluster_info, remove_node, write_json, write_node

from raftify.config import RaftifyConfig
from raftify.deserializer import init_rraft_py_deserializer
from raftify.log_entry.set_command import SetCommand
from raftify.peers import Peers, PeerState
from raftify.raft_client import RaftClient
from raftify.raft_facade import RaftFacade
from raftify.state_machine.hashstore import HashStore
from raftify.utils import SocketAddr

routes = RouteTableDef()


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
    cluster: RaftFacade = request.app["state"]["cluster"]
    id, value = request.match_info["id"], request.match_info["value"]
    message = SetCommand(id, value)
    result = await cluster.mailbox.send(message.encode())
    return web.Response(text=f'"{str(pickle.loads(result))}"')


@routes.get("/leave")
async def leave(request: web.Request) -> web.Response:
    cluster: RaftFacade = request.app["state"]["cluster"]
    id = cluster.raft_node.get_id()
    await cluster.mailbox.leave(id)
    return web.Response(text=f'Removed "node {id}" from the cluster successfully.')


@routes.get("/remove/{id}")
async def remove(request: web.Request) -> web.Response:
    cluster: RaftFacade = request.app["state"]["cluster"]
    id = int(request.match_info["id"])
    await cluster.mailbox.leave(id)
    return web.Response(text=f'Removed "node {id}" from the cluster successfully.')


def get_alive_node_ids(peers: Peers) -> list[int]:
    return [
        node_id for node_id, peer in peers.items() if peer.state == PeerState.Connected
    ]


@routes.get("/peers")
async def peers(request: web.Request) -> web.Response:
    cluster: RaftFacade = request.app["state"]["cluster"]
    return web.Response(text=str(cluster.peers))


@routes.get("/connected_nodes")
async def connected_nodes(request: web.Request) -> web.Response:
    cluster: RaftFacade = request.app["state"]["cluster"]
    return web.Response(text=json.dumps(get_alive_node_ids(cluster.peers)))


@routes.get("/leader")
async def leader(request: web.Request) -> web.Response:
    cluster: RaftFacade = request.app["state"]["cluster"]
    return web.Response(text=str(cluster.raft_node.get_leader_id()))


@actxmgr
async def server_main(
    _loop: asyncio.AbstractEventLoop, _pidx: int, args: list
) -> AsyncIterator[None]:
    """
    Raft server harness code using static membership.
    This will reduce the complexity and costs of the test code, and also could test static membership feature itself.
    """
    init_rraft_py_deserializer()

    peers: Peers = args[0]

    store = HashStore()
    raft_node_idx = process_index.get()
    node_id = raft_node_idx + 1
    raft_addr = SocketAddr.from_str(str(RAFT_ADDRS[raft_node_idx]))

    cfg = RaftifyConfig(
        log_dir="./logs",
        compacted_log_dir="./logs",
    )

    cluster = RaftFacade(cfg, raft_addr, store, slog, logger, peers)

    if raft_node_idx == 0:
        logger.info("Bootstrap a Raft Cluster")
        cluster.run_raft(node_id)
        asyncio.create_task(cluster.wait_for_followers_join())
        asyncio.create_task(cluster.wait_for_termination())
    else:
        # Wait for the leader node's grpc server ready
        await asyncio.sleep(2)
        logger.info("Running in follower mode")
        cluster.run_raft(node_id)

        leader_client = RaftClient(peers[1].addr)
        await leader_client.member_bootstrap_ready(node_id, timeout=5.0)
        asyncio.create_task(cluster.wait_for_termination())

    assert cluster.raft_node is not None, "RaftNode not initialized properly!"

    app = Application()
    app.add_routes(routes)
    app["state"] = {
        "store": store,
        "cluster": cluster,
    }

    runner = web.AppRunner(app)
    await runner.setup()
    host, port = WEB_SERVER_ADDRS[raft_node_idx].split(":")
    web_server = web.TCPSite(runner, host, port)

    asyncio.create_task(cluster.wait_for_termination())
    asyncio.create_task(web_server.start())

    def handle_sigterm(*args):
        remove_node(raft_node_idx + 1)
        sys.exit(0)

    signal.signal(signal.SIGTERM, handle_sigterm)

    write_node(raft_node_idx + 1, {"addr": str(raft_addr), "pid": os.getpid()})
    yield


async def excute_extra_node(node_id: int, raft_addr: SocketAddr, peers: Peers):
    init_rraft_py_deserializer()
    cfg = RaftifyConfig(
        log_dir="./logs",
        compacted_log_dir="./logs",
    )

    store = HashStore()
    cluster = RaftFacade(cfg, raft_addr, store, slog, logger, peers)
    peer_addrs = [peer.addr for peer in peers.values()]

    request_id_resp = await cluster.request_id(raft_addr, peer_addrs)
    cluster.run_raft(request_id_resp.follower_id)
    logger.info("Running in follower mode")

    app = Application()
    app.add_routes(routes)
    app["state"] = {
        "store": store,
        "cluster": cluster,
    }

    runner = web.AppRunner(app)
    await runner.setup()
    host, port = WEB_SERVER_ADDRS[node_id - 1].split(":")
    web_server = web.TCPSite(runner, host, port)

    await cluster.join_cluster(request_id_resp)

    def handle_sigterm(*args):
        remove_node(node_id)
        sys.exit(0)

    signal.signal(signal.SIGTERM, handle_sigterm)

    write_node(node_id, {"addr": str(raft_addr), "pid": os.getpid()})

    await asyncio.gather(cluster.wait_for_termination(), web_server.start())


def run_in_new_process(cb, *args):
    loop = asyncio.new_event_loop()
    asyncio.set_event_loop(loop)
    coro = cb(*args)
    task = asyncio.ensure_future(coro)
    try:
        loop.run_until_complete(task)
    finally:
        loop.close()


def spawn_extra_node(node_id: int, raft_addr: SocketAddr, peers: Peers):
    p = multiprocessing.Process(
        target=run_in_new_process, args=(excute_extra_node, node_id, raft_addr, peers)
    )
    p.start()


def run_raft_cluster(peers: Peers):
    write_json(f"{CLUSTER_INFO_PATH}/.root.json", {"pid": os.getpid()})

    try:
        aiotools.start_server(
            server_main,
            num_workers=len(peers),
            args=(peers,),
        )
    except Exception as e:
        print("Exception occurred!: ", e)
    finally:
        print("Terminated.")


async def wait_for_until(predicate: str, poll_interval: float = 1.0, end: float = 5.0):
    while True:
        nodes = read_cluster_info()["nodes"]
        cluster_size = len(nodes)

        if eval(predicate, {"cluster_size": cluster_size, "nodes": nodes}):
            break
        print(f'Waiting for cluster state changed to "{predicate}"...')
        await asyncio.sleep(poll_interval)

    print("Waiting for the conf_change reflected to the cluster...")
    await asyncio.sleep(end)
