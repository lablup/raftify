import asyncio
from concurrent.futures import ProcessPoolExecutor

import pytest
import requests

from tests.harness.constant import NODE_INFO_FILE_PATH, WEB_SERVER_ADDRS
from tests.harness.raft_server import run_raft_cluster
from tests.utils import kill_process, killall, prepare, read_json


@pytest.mark.asyncio
async def test_three_node_example():
    """
    Test the Raft implementation's resilience in a three-node example.

    This test simulates a scenario where the leader node in a three-node cluster is terminated.
    After the leader termination, it verifies that one of the remaining two nodes is elected
    as the new leader.

    Assumptions:
    - Nodes are up and running within 10 seconds of initialization.
    - A new leader is elected within 5 seconds of the original leader's termination.
    """

    prepare()
    loop = asyncio.get_running_loop()
    executor = ProcessPoolExecutor()
    loop.run_in_executor(executor, run_raft_cluster, (3))

    await asyncio.sleep(10)

    pids = read_json(NODE_INFO_FILE_PATH)["nodes"]

    # Kill the leader node
    kill_process(pids[0]["pid"])

    await asyncio.sleep(5)

    response = requests.get(f"http://{WEB_SERVER_ADDRS[1]}/leader")

    assert response.text == "2" or response.text == "3"

    await asyncio.sleep(1)

    killall()
    executor.shutdown()
    await asyncio.sleep(3)
