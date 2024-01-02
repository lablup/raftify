import asyncio
import pytest
from utils import load_peers, wait_for_until_cluster_size_increase
from constant import THREE_NODE_EXAMPLE
from harness.raft_server import RAFTS, handle_bootstrap, run_rafts, wait_for_termination
from harness.state_machine import SetCommand


@pytest.mark.asyncio
async def test_data_replication():
    peers = load_peers(THREE_NODE_EXAMPLE)
    asyncio.create_task(run_rafts(peers))
    asyncio.create_task(wait_for_termination())

    await asyncio.sleep(2)

    await handle_bootstrap(peers)
    await asyncio.sleep(2)

    raft_1 = RAFTS.get(1)
    await wait_for_until_cluster_size_increase(raft_1, 3)
    entry = SetCommand("1", "test")

    raft_node_1 = raft_1.get_raft_node()
    raft_node_1.prepare_proposal(entry.encode())
    await raft_node_1.propose()

    await asyncio.sleep(1)

    # Data should be replicated to all nodes.
    for raft in RAFTS.values():
        store = await raft.get_raft_node().store()
        assert store.get("1") == "test"

    for raft in RAFTS.values():
        await raft.get_raft_node().quit()
