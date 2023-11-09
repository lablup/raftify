import asyncio
import json
from concurrent.futures import ProcessPoolExecutor

import pytest
from constant import THREE_NODE_EXAMPLE
from harness.raft_server import run_raft_cluster, spawn_extra_node, wait_for_until
from utils import (
    RequestType,
    kill_node,
    killall,
    load_peers,
    make_request,
    reset_fixtures_directory,
)

from raftify.peers import Peer
from raftify.utils import SocketAddr


@pytest.mark.asyncio
async def test_disconnected_node_get_same_node_id():
    """
    Test a disconnected node gets same node_id when it reconnects.
    """

    reset_fixtures_directory()
    loop = asyncio.get_running_loop()
    executor = ProcessPoolExecutor()
    peers = load_peers(THREE_NODE_EXAMPLE)

    loop.run_in_executor(executor, run_raft_cluster, peers)
    await wait_for_until("cluster_size >= 3")

    peers_1 = json.loads(make_request(RequestType.GET, 1, "/connected_nodes"))
    assert peers_1 == [1, 2, 3]

    # Disconnect the node 2
    make_request(RequestType.GET, 1, "/remove/2")
    await kill_node(2)
    await wait_for_until("cluster_size <= 2")
    peers_states_2 = json.loads(make_request(RequestType.GET, 1, "/peers"))
    assert list(peers_states_2.keys()) == ["1", "2", "3"]
    assert peers_states_2["2"]["state"] != "Connected"

    peers_2 = json.loads(make_request(RequestType.GET, 1, "/connected_nodes"))
    assert peers_2 == [1, 3]

    # Reconnect to the node 2
    addr = SocketAddr(host="127.0.0.1", port=60062)
    peers[2] = Peer(addr)
    spawn_extra_node(2, addr, peers)
    await wait_for_until("cluster_size >= 3")

    peers_3 = json.loads(make_request(RequestType.GET, 1, "/connected_nodes"))
    assert peers_3 == [1, 2, 3]

    killall()
    executor.shutdown()


# TODO: Implement this test
@pytest.mark.asyncio
async def test_apply_confchange_forcely():
    """
    Test when a ConfChange should be committed even though the quorum is not satisfied
    """


# TODO: Implement this test
@pytest.mark.asyncio
async def test_cluster_merge():
    """ """


# TODO: Implement this test
@pytest.mark.asyncio
async def test_leader_transfer():
    """ """
