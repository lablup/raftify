import asyncio
import logging
from asyncio import Queue

import msgpack
from rraft import ConfChange, ConfChangeType, Logger_Ref

from riteraft.mailbox import Mailbox
from riteraft.protos import raft_service_pb2
from riteraft.raft_client import RaftClient
from riteraft.raft_node import RaftNode
from riteraft.raft_server import RaftServer
from riteraft.store import AbstractStore
from riteraft.utils import SocketAddr


class Raft:
    def __init__(self, addr: SocketAddr, store: AbstractStore, logger: Logger_Ref):
        """
        Creates a new node with the given address and store.
        """
        self.addr = addr
        self.store = store
        self.logger = logger
        self.chan = Queue(maxsize=100)

    def mailbox(self) -> Mailbox:
        """
        Get the node's `Mailbox`.
        """
        return Mailbox(self.chan)

    async def lead(self) -> None:
        """
        Create a new leader for the cluster, with id 1. There has to be exactly one node in the
        cluster that is initialized that way
        """
        raft_node = RaftNode.new_leader(self.chan, self.store, self.logger)
        server = RaftServer(self.addr, self.chan)
        asyncio.create_task(server.run())
        node_handle = asyncio.create_task(raft_node.run())
        await node_handle
        # TODO: Resolve several issues on the console at first...
        logging.warning("Leaving leader node")

    async def join(self, addr: SocketAddr) -> None:
        """
        Try to join a new cluster at `addr`, getting an id from the leader, or finding it if
        `addr` is not the current leader of the cluster
        """

        # 1. try to discover the leader and obtain an id from it.
        logging.info(f"Attempting to join peer cluster at {str(addr)}")
        leader_addr = str(addr)

        leader_id, node_id = None, None
        while True:
            client = RaftClient(addr)
            resp = await client.request_id()

            if resp.code == raft_service_pb2.Ok:
                leader_id, node_id = msgpack.unpackb(resp.data)
                break
            elif resp.code == raft_service_pb2.WrongLeader:
                _, leader_addr = msgpack.unpackb(resp.data)
                logging.info(f"Wrong leader, retrying with leader at {leader_addr}")
                continue
            elif resp.code == raft_service_pb2.Error:
                logging.error("Error joining the cluster")
                return

        logging.info(f"Obtained ID from leader: {node_id}")

        # 2. run server and node to prepare for joining
        raft_node = RaftNode.new_follower(self.chan, node_id, self.store, self.logger)
        client = RaftClient(leader_addr)
        raft_node.peers[leader_id] = client
        server = RaftServer(self.addr, self.chan)
        asyncio.create_task(server.run())
        raft_node_handle = asyncio.create_task(raft_node.run())

        # 3. Join the cluster
        # TODO: handle wrong leader
        change = ConfChange.default()
        change.set_node_id(node_id)
        change.set_change_type(ConfChangeType.AddNode)
        change.set_context(msgpack.packb(self.addr))

        await client.change_config(change)
        await raft_node_handle
