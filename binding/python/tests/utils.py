from asyncio import sleep
import os
import tomli
from pathlib import Path
from raftify import InitialRole, Peer, Peers, Raft


def load_peers(filename: str) -> Peers:
    path = Path(__file__).parent / "fixtures" / filename
    cfg = tomli.loads(path.read_text())["raft"]["peers"]

    return Peers(
        {
            int(entry["node_id"]): Peer(
                addr=f"{entry['ip']}:{entry['port']}",
                role=InitialRole.from_str(entry["role"]),
            )
            for entry in cfg
        }
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


def get_storage_path(log_dir: str, node_id: int) -> str:
    return f"{log_dir}/node-{node_id}"


def ensure_directory_exist(storage_path: str):
    if not os.path.exists(storage_path):
        os.makedirs(storage_path)
