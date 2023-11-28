# This file is a simple shortcut example script for only development purpose.
# If you need example code, please refer to https://github.com/lablup/raftify/tree/main/examples/basic.

import argparse
import asyncio
from contextlib import suppress

from raftify.rraft_deserializer import init_rraft_py_deserializer
from raftify.raft_facade import RaftFacade
from raftify.state_machine.hashstore import HashStore
from raftify.utils import SocketAddr

from examples.basic.logger import logger, slog
from examples.basic.utils import WebServer, build_config, load_peers
from examples.basic.web_server import routes


async def main() -> None:
    init_rraft_py_deserializer()

    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--bootstrap", action=argparse.BooleanOptionalAction, default=None
    )
    parser.add_argument("--raft-addr", default=None)
    parser.add_argument("--web-server", default=None)
    args = parser.parse_args()
    bootstrap = args.bootstrap
    raft_addr = (
        SocketAddr.from_str(args.raft_addr) if args.raft_addr is not None else None
    )
    web_server_addr = args.web_server
    peers = load_peers()
    store = HashStore()
    target_addr = peers[1].addr if bootstrap and not raft_addr else raft_addr

    cfg = build_config()
    raft = RaftFacade(cfg, target_addr, store, slog, logger, peers)
    tasks = []

    if bootstrap:
        logger.info("Bootstrap a Raft Cluster")
        node_id = 1
        raft.run_raft(node_id)
        tasks.append(raft.wait_for_followers_join())
        tasks.append(raft.wait_for_termination())
    else:
        assert (
            raft_addr is not None
        ), "Follower node requires a --raft-addr option to join the cluster"

        logger.info("Running in follower mode")
        node_id = peers.get_node_id_by_addr(raft_addr)
        assert node_id is not None, "Member Node id is not found"

        raft.run_raft(node_id)
        await raft.send_member_bootstrap_ready_msg(node_id)
        tasks.append(raft.wait_for_termination())

    async with WebServer(web_server_addr, routes, {"raft": raft}):
        await asyncio.gather(*tasks)


if __name__ == "__main__":
    with suppress(KeyboardInterrupt):
        asyncio.run(main())
