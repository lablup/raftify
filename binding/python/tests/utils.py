from asyncio import sleep
import tomli
from pathlib import Path
from raftify import Peers, Raft


def load_peers(filename: str) -> Peers:
    path = Path(__file__).parent / "fixtures" / filename
    cfg = tomli.loads(path.read_text())["raft"]["peers"]

    return Peers(
        {int(entry["node_id"]): f"{entry['host']}:{entry['port']}" for entry in cfg}
    )


async def wait_for_until_cluster_size_increase(raft: Raft, target: int):
    print("Waiting for cluster size to increase to {}...", target)

    while True:
        size = await raft.get_raft_node().get_cluster_size()
        if size >= target:
            break

        await sleep(0.1)

    # Wait for the conf_change reflected to the cluster
    await sleep(0.1)


async def wait_for_until_cluster_size_decrease(raft: Raft, target: int):
    print("Waiting for cluster size to decrease to {}...", target)

    while True:
        size = await raft.get_raft_node().get_cluster_size()
        if size <= target:
            break

        await sleep(0.1)

    # Wait for the conf_change reflected to the cluster
    await sleep(0.1)
