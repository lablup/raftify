import asyncio
import os
from harness.state_machine import HashStore
from raftify import Config, Peers, Raft, RaftConfig, Logger


RAFTS: dict[int, Raft] = {}


def build_config() -> Config:
    raft_cfg = RaftConfig()
    cfg = Config(
        raft_cfg,
        log_dir="./logs",
        compacted_log_dir="./logs",
    )

    return cfg


async def wait_for_termination():
    for raft in RAFTS.values():
        while not raft.is_finished():
            await asyncio.sleep(1.0)


async def run_raft(node_id: int, peers: Peers):
    peer = peers.get(node_id)
    cfg = build_config()

    store = HashStore()
    logger = Logger.default()
    raft = Raft.build(node_id, peer, store, cfg, logger, peers)
    await raft.run()

    RAFTS[node_id] = raft


async def run_rafts(peers: Peers):
    # TODO: This should be configurable
    os.environ["RUST_LOG"] = "debug"

    for node_id, _ in peers.items():
        await run_raft(node_id, peers)


async def handle_bootstrap(peers: Peers):
    leader_addr = peers.get(1)

    for (node_id, _) in peers.items():
        if node_id != 1:
            raft = RAFTS.get(node_id)
            raft.prepare_member_bootstrap_ready(leader_addr, node_id)
            await raft.member_bootstrap_ready()
