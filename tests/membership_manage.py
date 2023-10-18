import asyncio
import json
from concurrent.futures import ProcessPoolExecutor

import pytest
from harness.raft_server import run_raft_cluster, spawn_extra_node, wait_for_until
from raftify.peers import Peer
from raftify.raft_client import RaftClient
from raftify.utils import SocketAddr
from utils import RequestType, kill_node, killall, load_peers, make_request, reset_fixtures_directory


@pytest.mark.asyncio
async def test_disconnected_node_get_same_node_id():
    """
    Test a disconnected node gets same node_id when it reconnects.
    """

    reset_fixtures_directory()
    loop = asyncio.get_running_loop()
    executor = ProcessPoolExecutor()
    peers = load_peers("3-node-example.toml")

    loop.run_in_executor(executor, run_raft_cluster, peers)
    await wait_for_until("cluster_size >= 3")

    peers_1 = json.loads(make_request(RequestType.GET, 1, "/connected_nodes"))
    assert peers_1 == [1, 2, 3]

    # Disconnect the node 2
    make_request(RequestType.GET, 1, "/remove/2")
    await kill_node(2)
    await wait_for_until("cluster_size <= 2")
    peers_2 = json.loads(make_request(RequestType.GET, 1, "/connected_nodes"))
    assert peers_2 == [1, 3]

    # Reconnect to the node 2
    addr = SocketAddr(host="127.0.0.1", port=60062)
    peers[2] = Peer(addr, client=RaftClient(addr))
    spawn_extra_node(2, addr, peers)
    await wait_for_until("cluster_size >= 3")

    peers_3 = json.loads(make_request(RequestType.GET, 1, "/connected_nodes"))
    assert peers_3 == [1, 2, 3]

    killall()
    executor.shutdown()
