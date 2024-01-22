import asyncio
import os
from typing import Any
from harness.state_machine import HashStore
from raftify import Config, Peers, Raft, RaftConfig, Slogger


RAFTS: dict[int, Raft] = {}


def build_config() -> Config:
    raft_cfg = RaftConfig(
        election_tick=10,
        heartbeat_tick=3,
    )
    cfg = Config(
        raft_cfg,
        log_dir="./logs",
        compacted_log_dir="./logs",
    )

    return cfg


async def run_raft(node_id: int, peers: Peers):
    peer = peers.get(node_id)
    cfg = build_config()

    store = HashStore()
    logger = Slogger.default()

    if node_id == 1:
        raft = Raft.bootstrap_cluster(node_id, peer, store, cfg, logger, peers)
    else:
        raft = Raft.new_follower(node_id, peer, store, cfg, logger, peers)

    RAFTS[node_id] = raft

    await raft.run()


async def run_rafts(peers: Peers):
    # TODO: This should be configurable
    os.environ["RUST_LOG"] = "debug"

    tasks = []
    for node_id, _ in peers.items():
        tasks.append(run_raft(node_id, peers))

    await asyncio.gather(*tasks)


async def handle_bootstrap(peers: Peers, logger: Any):
    leader_addr = peers.get(1)

    for (node_id, _) in peers.items():
        if node_id != 1:
            await Raft.member_bootstrap_ready(leader_addr, node_id, logger)
