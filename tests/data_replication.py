import asyncio
from concurrent.futures import ProcessPoolExecutor

import pytest
from harness.raft_server_v2 import run_raft_cluster, wait_for_until
from utils import RequestType, killall, make_request, reset_fixtures_directory


@pytest.mark.asyncio
async def test_data_replication():
    """
    This test simulates a scenario where the data replication execute.
    Data should be replicated to all nodes.
    """

    reset_fixtures_directory()
    loop = asyncio.get_running_loop()
    executor = ProcessPoolExecutor()
    loop.run_in_executor(executor, run_raft_cluster, (3))
    await wait_for_until("cluster_size >= 3")

    make_request(RequestType.GET, 1, "/put/1/A")
    await asyncio.sleep(3)

    for i in range(1, 4):
        resp = make_request(RequestType.GET, i, "/get/1")
        assert resp == "A"

    # Check if the leader is one of the remaining two nodes
    assert resp == "A"

    killall()
    executor.shutdown()
