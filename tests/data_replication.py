import asyncio
from concurrent.futures import ProcessPoolExecutor

import pytest
from harness.raft_server import run_raft_cluster, spawn_extra_node, wait_for_until
from raftify.peers import Peer
from raftify.raft_client import RaftClient
from raftify.utils import SocketAddr
from utils import RequestType, killall, load_peers, make_request, reset_fixtures_directory


@pytest.mark.asyncio
async def test_data_replication():
    """
    This test simulates a scenario where the data replication execute.
    Data should be replicated to all nodes.
    """

    reset_fixtures_directory()
    loop = asyncio.get_running_loop()
    executor = ProcessPoolExecutor()
    peers = load_peers("3-node-example.toml")

    loop.run_in_executor(executor, run_raft_cluster, peers)
    await wait_for_until("cluster_size >= 3")

    make_request(RequestType.GET, 1, "/put/1/A")
    await asyncio.sleep(3)

    # Data should be replicated to all nodes.
    for i in range(1, 4):
        resp = make_request(RequestType.GET, i, "/get/1")
        assert resp == "A"

    addr = SocketAddr(host="127.0.0.1", port=60064)
    peers[4] = Peer(addr, client=RaftClient(addr))

    spawn_extra_node(4, addr, peers)
    await wait_for_until("cluster_size >= 4")

    # Data should be replicated to new joined node.
    resp = make_request(RequestType.GET, 4, "/get/1")
    assert resp == "A"

    killall()
    executor.shutdown()
