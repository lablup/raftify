import asyncio
from threading import Thread

from aiohttp.web import RouteTableDef
from raftify.config import RaftifyConfig
from raftify.deserializer import init_rraft_py_deserializer
from raftify.raft_client import RaftClient

from raftify.raft_cluster import RaftCluster
from raftify.peers import Peer, Peers
from raftify.utils import SocketAddr
from tests.harness.logger import logger, slog
from tests.harness.store import HashStore

import logging
routes = RouteTableDef()

logging.basicConfig(level=logging.DEBUG)


class RaftThreads:
    peer_addrs = [
        {"ip": "127.0.0.1", "port": 60061, "node_id": 1},
        {"ip": "127.0.0.1", "port": 60062, "node_id": 2},
        {"ip": "127.0.0.1", "port": 60063, "node_id": 3},
    ]

    cfg = RaftifyConfig(
        log_dir="./",
        raft_config=RaftifyConfig.new_raft_config(
            {
                "election_tick": 10,
                "heartbeat_tick": 3,
            }
        ),
    )

    def __init__(self):
        init_rraft_py_deserializer()

        num_threads = len(self.peer_addrs)
        self.threads = []

        self.peers = Peers(
            {
                int(entry["node_id"]): Peer(addr=SocketAddr(entry["ip"], entry["port"]))
                for entry in self.peer_addrs
            }
        )

        for i in range(num_threads):
            th = Thread(target=self.run_async_in_thread, args=(i,))
            self.threads.append(th)

        for i in range(num_threads):
            self.threads[i].start()

        for i in range(num_threads):
            self.threads[i].join()

        logger.info('All threads joined!')

    def run_async_in_thread(self, worker_index):
        loop = asyncio.new_event_loop()
        asyncio.set_event_loop(loop)
        loop.run_until_complete(self.run_raft(worker_index))

    async def run_raft(self, worker_index: int):
        bootstrap = worker_index == 0

        worker = self.peer_addrs[worker_index]

        raft_addr = SocketAddr(worker["ip"], worker["port"])

        store = HashStore()
        cluster = RaftCluster(self.cfg, raft_addr, store, slog, logger, self.peers)
        tasks = []

        if bootstrap:
            logger.info("Bootstrap a Raft Cluster")
            node_id = 1
            cluster.run_raft(node_id)
            tasks.append(cluster.wait_for_termination())
            tasks.append(cluster.join_all_followers())
        else:
            await asyncio.sleep(2 * worker_index)
            # await asyncio.sleep(2)
            node_id = worker["node_id"]
            cluster.run_raft(node_id)
            leader_client = RaftClient(self.peers[1].addr)

            await leader_client.member_bootstrap_ready(node_id, 5.0)
            tasks.append(cluster.wait_for_termination())

        await asyncio.gather(*tasks)
