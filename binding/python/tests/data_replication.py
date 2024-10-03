import asyncio
import pytest
from utils import (
    cleanup_storage,
    load_peers,
    wait_for_until_cluster_size_increase,
)
from constant import THREE_NODE_EXAMPLE
from harness.raft_server import RAFTS, run_rafts
from harness.state_machine import SetCommand


@pytest.mark.asyncio
async def test_data_replication():
    cleanup_storage("./logs")

    peers = load_peers(THREE_NODE_EXAMPLE)
    asyncio.create_task(run_rafts(peers))
    await asyncio.sleep(2)

    raft_1 = RAFTS.get(1)
    await wait_for_until_cluster_size_increase(raft_1, 3)
    entry = SetCommand("1", "test")

    raft_node_1 = raft_1.get_raft_node()
    await raft_node_1.propose(entry.encode())

    await asyncio.sleep(1)

    # Data should be replicated to all nodes.
    for raft in RAFTS.values():
        state_machine = await raft.get_raft_node().state_machine()
        assert state_machine.get("1") == "test"

    for raft in RAFTS.values():
        await raft.get_raft_node().quit()
