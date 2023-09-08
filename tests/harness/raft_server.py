import asyncio
import json
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

from raftify.error import ClusterJoinError, LeaderNotFoundError
from raftify.raft_facade import RaftCluster, RaftNodeRole
from raftify.utils import SocketAddr
from tests.harness.constant import CLUSTER_INFO_PATH, RAFT_ADDRS, WEB_SERVER_ADDRS
from tests.harness.log import SetCommand
from tests.harness.logger import logger, slog
from tests.harness.store import HashStore
from tests.utils import get_cluster_size, remove_node, write_json, write_node

routes = RouteTableDef()


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


@actxmgr
async def server_main(
    loop: asyncio.AbstractEventLoop, pidx: int, _args: list
) -> AsyncIterator[None]:
    store = HashStore()
    raft_node_idx = process_index.get()
    raft_socket = SocketAddr.from_str(str(RAFT_ADDRS[raft_node_idx]))
    cluster = RaftCluster(raft_socket, store, slog, logger)

    if raft_node_idx == 0:
        cluster.build_raft(RaftNodeRole.Leader)
        cluster.bootstrap_cluster()
    else:
        # TODO: Handle waiting time more properly if it could be.
        await asyncio.sleep(3.0 * (raft_node_idx + 1))

        while True:
            print("Trying to join cluster...")
            try:
                request_id_response = await cluster.request_id(
                    raft_socket,
                    [SocketAddr.from_str(str(raft_addr)) for raft_addr in RAFT_ADDRS],
                )
            except LeaderNotFoundError:
                print("Leader not found! retry after 2s...")
                await asyncio.sleep(2)
                continue

            cluster.build_raft(RaftNodeRole.Follower, request_id_response.follower_id)

            try:
                await cluster.join_cluster(request_id_response)
                break
            except ClusterJoinError as e:
                print("ClusterJoinError! retry after 2s...: ", e)
                await asyncio.sleep(2)
                continue

    assert cluster.raft_node is not None, "RaftNode not initialized properly!"

    app = Application()
    app.add_routes(routes)
    app["state"] = {
        "store": store,
        "cluster": cluster,
    }

    def handle_sigterm(*args):
        remove_node(raft_node_idx + 1)
        sys.exit(0)

    signal.signal(signal.SIGTERM, handle_sigterm)

    runner = web.AppRunner(app)
    await runner.setup()
    host, port = WEB_SERVER_ADDRS[raft_node_idx].split(":")
    web_server = web.TCPSite(runner, host, port)

    asyncio.create_task(cluster.run_raft())
    asyncio.create_task(web_server.start())

    write_node(raft_node_idx + 1, {"addr": str(raft_socket), "pid": os.getpid()})
    yield


def run_raft_cluster(num_workers: int):
    write_json(f"{CLUSTER_INFO_PATH}/.root.json", {"pid": os.getpid()})

    try:
        aiotools.start_server(
            server_main,
            num_workers=num_workers,
        )
    except Exception as e:
        print("Exception occurred!: ", e)
    finally:
        print("Terminated.")


async def wait_for_cluster_change(
    predicate: str, poll_interval: float = 1.0, end: float = 3.0
):
    while True:
        cluster_size = get_cluster_size()
        if eval(predicate, {"cluster_size": cluster_size}):
            break
        print(f'Waiting for cluster state changed to "{predicate}"...')
        await asyncio.sleep(poll_interval)

    print("Waiting for the confchange reflected to the cluster...")
    await asyncio.sleep(end)
