import asyncio
import logging
import pickle
from asyncio import Queue

import grpc
from rraft import ConfChange, ConfChangeType, Logger, LoggerRef

from riteraft.error import ClusterJoinError, UnknownError
from riteraft.fsm import FSM
from riteraft.mailbox import Mailbox
from riteraft.protos import raft_service_pb2
from riteraft.raft_client import RaftClient
from riteraft.raft_node import RaftNode
from riteraft.raft_server import RaftServer
from riteraft.utils import SocketAddr


class RaftClusterFacade:
    def __init__(self, addr: SocketAddr, fsm: FSM, logger: Logger | LoggerRef):
        """
        Creates a new node with the given address and store.
        """
        self.addr = addr
        self.fsm = fsm
        self.logger = logger
        self.chan = Queue(maxsize=100)
        self.raft_node = None

    def mailbox(self) -> Mailbox:
        """
        Get the node's `Mailbox`.
        """
        return Mailbox(self.chan)

    def get_peers(self) -> dict[int, RaftClient]:
        return self.raft_node.peers

    async def bootstrap_cluster(self) -> None:
        """
        Create a new leader for the cluster, with id 1. There has to be exactly one node in the
        cluster that is initialized that way
        """
        asyncio.create_task(RaftServer(self.addr, self.chan).run())
        self.raft_node = RaftNode.bootstrap_leader(self.chan, self.fsm, self.logger)
        await asyncio.create_task(self.raft_node.run())
        logging.warning("Leaving leader node")

    async def join_cluster(self, peer_candidates: list[SocketAddr]) -> None:
        """
        Try to join a new cluster at `peer_addr`, getting an id from the leader, or finding it if
        `peer_addr` is not the current leader of the cluster
        """

        # 1. Discover the leader of the cluster and obtain an 'node_id'.
        for peer_addr in peer_candidates:
            logging.info(f'Attempting to join peer cluster at "{peer_addr}"')

            leader_addr = None
            seek_next = False

            while not leader_addr:
                client = RaftClient(peer_addr)
                try:
                    resp = await client.request_id(peer_addr)
                except grpc.aio.AioRpcError:
                    seek_next = True
                    break

                match resp.code:
                    case raft_service_pb2.Ok:
                        leader_addr = peer_addr
                        leader_id, node_id, peer_addrs = pickle.loads(resp.data)
                        break
                    case raft_service_pb2.WrongLeader:
                        _, peer_addr, _ = pickle.loads(resp.data)
                        logging.info(
                            f"Wrong leader, retrying with leader at {peer_addr}"
                        )
                        continue
                    case raft_service_pb2.Error | _:
                        raise UnknownError("Failed to join the cluster!")

            if seek_next:
                continue
            break
        else:
            raise ClusterJoinError(
                "Could not join the cluster. Check your raft configuration and check to make sure that any of them is alive."
            )

        assert leader_id is not None and node_id is not None

        logging.info(f"Obtained node id {node_id} from the leader {leader_id}.")

        # 2. Run server and node to prepare for joining
        self.raft_node = RaftNode.new_follower(
            self.chan, node_id, self.fsm, self.logger
        )

        self.raft_node.peers = {
            node_id: RaftClient(addr) for node_id, addr in peer_addrs.items()
        }
        self.raft_node.peers[leader_id] = client

        _ = asyncio.create_task(RaftServer(self.addr, self.chan).run())
        raft_node_handle = asyncio.create_task(self.raft_node.run())

        # 3. Join the cluster
        conf_change = ConfChange.default()
        conf_change.set_node_id(node_id)
        conf_change.set_change_type(ConfChangeType.AddNode)
        conf_change.set_context(pickle.dumps(self.addr))

        # TODO: Should handle wrong leader error here because the leader might change in the meanwhile.
        await client.change_config(conf_change)
        await raft_node_handle
