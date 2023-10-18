import asyncio
from concurrent.futures import ProcessPoolExecutor
import json

import pytest
from harness.raft_server import run_raft_cluster, wait_for_until
from utils import RequestType, kill_node, killall, load_peers, make_request, reset_fixtures_directory


# TODO: Maybe it would be better to replace RerouteMessage with network layer's implementation for simplicity.
@pytest.mark.asyncio
async def test_message_rerouting():
    """
    """

    reset_fixtures_directory()
    loop = asyncio.get_running_loop()
    executor = ProcessPoolExecutor()
    peers = load_peers("3-node-example.toml")

    loop.run_in_executor(executor, run_raft_cluster, peers)
    await wait_for_until("cluster_size >= 3")

    # Make EntryNormal request to the follower node.
    make_request(RequestType.GET, 2, "/put/1/A")

    await asyncio.sleep(3)

    # Data should be replicated to all nodes.
    for i in range(1, 4):
        resp = make_request(RequestType.GET, i, "/get/1")
        assert resp == "A"

    # Make EntryConfChange request to the follower node.
    make_request(RequestType.GET, 2, "/remove/3")

    await kill_node(3)
    await wait_for_until("cluster_size <= 2")
    peers = json.loads(make_request(RequestType.GET, 1, "/connected_nodes"))
    assert peers == [1, 2]

    killall()
    executor.shutdown()


# TODO: Implement this test
@pytest.mark.asyncio
async def test_serial_confchanges():
    """
    Test that creating a lot of confchanges at the same time does not create a pending_conf_change.
    """
