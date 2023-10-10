from asyncio import sleep
import logging
import pytest
from harness.raft_server import RaftThreads

logging.getLogger("faker").setLevel(logging.WARNING)


@pytest.mark.asyncio
async def test_leader_election_three_node_example():
    """
    This test simulates a scenario where the leader node in a three-node cluster is terminated.
    After the leader termination, it verifies that one of the remaining two nodes is elected
    as the new leader.
    """

    RaftThreads()

    # await sleep(10)
