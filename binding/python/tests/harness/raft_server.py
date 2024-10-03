import asyncio
from harness.state_machine import HashStore
from raftify import Config, Peers, Raft, RaftConfig, Slogger
from .utils import ensure_directory_exist, get_storage_path


# All Raft objects per node ID.
RAFTS: dict[int, Raft] = {}


def build_config(node_id: int, initial_peers: Peers) -> Config:
    raft_cfg = RaftConfig(
        id=node_id,
        election_tick=10,
        heartbeat_tick=3,
    )

    storage_path = get_storage_path("./logs", node_id)
    ensure_directory_exist(storage_path)

    cfg = Config(
        raft_cfg,
        log_dir=storage_path,
        compacted_log_dir=storage_path,
        initial_peers=initial_peers,
    )

    return cfg


async def run_raft(node_id: int, initial_peers: Peers):
    peer = initial_peers.get(node_id)
    cfg = build_config(node_id, initial_peers)

    store = HashStore()
    logger = Slogger.default()
    raft = Raft.bootstrap(node_id, peer.get_addr(), store, cfg, logger)

    RAFTS[node_id] = raft

    await raft.run()


async def run_rafts(peers: Peers):
    tasks = []
    for node_id in peers.keys():
        tasks.append(run_raft(node_id, peers))

    await asyncio.gather(*tasks)
