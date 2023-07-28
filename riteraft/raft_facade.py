import asyncio
import logging
import pickle
from asyncio import Queue
from enum import Enum

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


class FollowerRole(Enum):
    Voter = ConfChangeType.AddNode
    Learner = ConfChangeType.AddLearnerNode

    def to_changetype(self) -> ConfChangeType:
        match self.value:
            case ConfChangeType.AddNode:
                return ConfChangeType.AddNode
            case ConfChangeType.AddLearnerNode:
                return ConfChangeType.AddLearnerNode
            case _:
                assert "Unreachable"


class RaftCluster:
    def __init__(self, addr: SocketAddr, fsm: FSM, logger: Logger | LoggerRef):
        """
        Creates a new node with the given address and store.
        """
        self.addr = addr
        self.fsm = fsm
        self.logger = logger
        self.chan = Queue(maxsize=100)
        self.raft_node = None

    @property
    def mailbox(self) -> Mailbox:
        """
        Get the node's `Mailbox`.
        """
        return Mailbox(self.raft_node, self.chan)

    def get_peers(self) -> dict[int, RaftClient]:
        return self.raft_node.peers

    async def bootstrap_cluster(self) -> None:
        """
        Create a new leader for the cluster with node id 1.
        """
        asyncio.create_task(RaftServer(self.addr, self.chan).run())
        self.raft_node = RaftNode.bootstrap_leader(self.chan, self.fsm, self.logger)
        await asyncio.create_task(self.raft_node.run())

    async def join_cluster(
        self,
        raft_addr: SocketAddr,
        peer_candidates: list[SocketAddr],
        role: FollowerRole = FollowerRole.Voter,
    ) -> None:
        """
        Try to join a new cluster through `peer_candidates` and get `node id` from the cluster's leader.
        """

        # 1. Discover the leader of the cluster and obtain an 'node_id'.
        for peer_addr in peer_candidates:
            logging.info(f'Attempting to join the cluster through "{peer_addr}"...')

            leader_addr = None
            seek_next = False

            while not leader_addr:
                client = RaftClient(peer_addr)
                try:
                    resp = await client.request_id(raft_addr)
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
                            f"Sent message to the wrong leader, retrying with the leader at {peer_addr}"
                        )
                        continue
                    case raft_service_pb2.Error | _:
                        raise UnknownError("Failed to join the cluster!")

            if not seek_next:
                break
        else:
            raise ClusterJoinError(
                "Could not join the cluster. Check your raft configuration and check to make sure that any of them is alive."
            )

        assert leader_id is not None and node_id is not None

        logging.info(
            f"Cluster join succeeded. Obtained node id {node_id} from the leader node {leader_id}."
        )

        # 2. Run server and node to prepare for joining
        self.raft_node = RaftNode.new_follower(
            self.chan, node_id, self.fsm, self.logger
        )

        self.raft_node.peers = {
            **{node_id: RaftClient(addr) for node_id, addr in peer_addrs.items()},
            leader_id: client,
        }

        asyncio.create_task(RaftServer(self.addr, self.chan).run())
        raft_node_handle = asyncio.create_task(self.raft_node.run())

        # 3. Join the cluster
        conf_change = ConfChange.default()
        conf_change.set_node_id(node_id)
        conf_change.set_change_type(role.to_changetype())
        conf_change.set_context(pickle.dumps(self.addr))

        # TODO: Should handle wrong leader error here because the leader might change in the meanwhile.
        await client.change_config(conf_change)
        await raft_node_handle
