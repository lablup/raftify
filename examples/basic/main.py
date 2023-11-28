import argparse
import asyncio
from contextlib import suppress

from raftify.deserializer import init_rraft_py_deserializer
from raftify.peers import Peers
from raftify.raft_client import RaftClient
from raftify.raft_facade import RaftFacade
from raftify.state_machine.hashstore import HashStore
from raftify.utils import SocketAddr

from .logger import logger, slog
from .utils import WebServer, build_config, load_peer_candidates, load_peers
from .web_server import routes


async def main() -> None:
    """
    Usage:
        main.py [--bootstrap] [--bootstrap-follower] [--raft-addr=<addr>] [--web-server=<addr>]
    Example:
        # Bootstrap Raft Cluster
        python -m examples.basic.main --bootstrap --web-server=127.0.0.1:8001

        # Initial nodes request to join the cluster, and then the cluster starts up.
        python -m examples.basic.main --bootstrap-follower --raft-addr=127.0.0.1:60062 --web-server=127.0.0.1:8002
        python -m examples.basic.main --bootstrap-follower --raft-addr=127.0.0.1:60063 --web-server=127.0.0.1:8003

        # You can join the extra node after the cluster bootstrapping is done.
        python -m examples.basic.main --raft-addr=127.0.0.1:60064 --web-server=127.0.0.1:8004
    """

    init_rraft_py_deserializer()

    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--bootstrap", action=argparse.BooleanOptionalAction, default=None
    )
    parser.add_argument(
        "--bootstrap-follower", action=argparse.BooleanOptionalAction, default=None
    )
    parser.add_argument("--raft-addr", default=None)
    parser.add_argument("--web-server", default=None)
    args = parser.parse_args()
    bootstrap = args.bootstrap
    bootstrap_follower = args.bootstrap_follower
    raft_addr = (
        SocketAddr.from_str(args.raft_addr) if args.raft_addr is not None else None
    )
    web_server_addr = args.web_server
    peers = (
        load_peers() if (raft_addr and bootstrap_follower) or bootstrap else Peers({})
    )
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
    elif bootstrap_follower:
        assert raft_addr is not None

        logger.info("Running in follower bootstrap mode")
        node_id = peers.get_node_id_by_addr(raft_addr)
        assert node_id is not None, "Member Node id is not found"

        raft.run_raft(node_id)
        leader_client = RaftClient(peers[1].addr)
        await leader_client.member_bootstrap_ready(node_id)
        tasks.append(raft.wait_for_termination())
    else:
        # Extra follower could only join after cluster bootstrapping is done
        assert raft_addr is not None

        logger.info("Running in follower mode")

        peer_addrs = load_peer_candidates()
        request_id_resp = await raft.request_id(raft_addr, peer_addrs)
        raft.run_raft(request_id_resp.follower_id)
        await raft.join_cluster(request_id_resp)
        tasks.append(raft.wait_for_termination())

    async with WebServer(web_server_addr, routes, {"raft": raft}):
        await asyncio.gather(*tasks)


if __name__ == "__main__":
    with suppress(KeyboardInterrupt):
        asyncio.run(main())
