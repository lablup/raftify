import asyncio

from examples.basic.logger import logger, slog
from examples.basic.utils import WebServer, build_config, load_peers
from examples.basic.web_server import routes
from raftify.cli import AbstractCLIContext
from raftify.raft_facade import RaftFacade
from raftify.rraft_deserializer import init_rraft_py_deserializer
from raftify.state_machine.hashstore import HashStore
from raftify.utils import SocketAddr


class RaftifyCLIContext(AbstractCLIContext):
    """
    Define the abstract raftify-cli commands.
    """

    def __init__(self) -> None:
        super().__init__()
        init_rraft_py_deserializer()

    async def bootstrap_cluster(self, _args, options):
        web_server_addr = options.get("web_server")

        leader_node_id = 1
        initial_peers = load_peers()
        leader_addr = initial_peers[leader_node_id].addr

        store = HashStore()
        cfg = build_config()
        raft = RaftFacade(cfg, leader_addr, store, slog, logger, initial_peers)

        logger.info("Bootstrapping Raft Cluster...")
        raft.run_raft(node_id=1)

        async with WebServer(web_server_addr, routes, {"raft": raft}):
            await asyncio.gather(
                raft.wait_for_followers_join(),
                raft.wait_for_termination(),
            )

    async def bootstrap_follower(self, _args, options):
        raft_addr = SocketAddr.from_str(options["raft_addr"])
        web_server_addr = options["web_server"]

        initial_peers = load_peers()
        store = HashStore()
        cfg = build_config()
        raft = RaftFacade(cfg, raft_addr, store, slog, logger, initial_peers)

        logger.info("Participating in Raft Cluster...")
        node_id = initial_peers.get_node_id_by_addr(raft_addr)
        assert node_id is not None, "Member node_id is not found"

        raft.run_raft(node_id)
        await raft.send_member_bootstrap_ready_msg(node_id)

        async with WebServer(web_server_addr, routes, {"raft": raft}):
            await asyncio.gather(
                raft.wait_for_termination(),
            )

    async def add_member(self, _args, options):
        raft_addr = SocketAddr.from_str(options["raft_addr"])
        web_server_addr = options["web_server"]

        initial_peers = load_peers()
        peer_addrs = list(map(lambda peer: peer.addr, initial_peers))

        store = HashStore()
        cfg = build_config()
        raft = RaftFacade(cfg, raft_addr, store, slog, logger)

        request_id_resp = await raft.request_id(raft_addr, peer_addrs)
        raft.run_raft(request_id_resp.follower_id)
        logger.info("Running in follower mode...")
        await raft.join_cluster(request_id_resp)

        async with WebServer(web_server_addr, routes, {"raft": raft}):
            await asyncio.gather(
                raft.wait_for_termination(),
            )
