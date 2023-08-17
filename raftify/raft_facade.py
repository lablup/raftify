import asyncio
import pickle
from asyncio import Queue
from enum import Enum

import grpc
from rraft import ConfChange, ConfChangeType, Logger, LoggerRef

from raftify.config import RaftConfig
from raftify.error import ClusterJoinError, UnknownError
from raftify.fsm import FSM
from raftify.logger import AbstractRaftifyLogger
from raftify.mailbox import Mailbox
from raftify.protos import raft_service_pb2
from raftify.raft_client import RaftClient
from raftify.raft_node import RaftNode
from raftify.raft_server import RaftServer
from raftify.utils import SocketAddr


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
    cluster_config = RaftConfig()

    def __init__(
        self,
        addr: SocketAddr,
        fsm: FSM,
        slog: Logger | LoggerRef,
        logger: AbstractRaftifyLogger,
    ):
        """
        Creates a new node with the given address and store.
        """
        self.addr = addr
        self.fsm = fsm
        self.slog = slog
        self.logger = logger
        self.chan = Queue(maxsize=100)
        self.raft_node = None

    @property
    def mailbox(self) -> Mailbox:
        """
        Get the node's `Mailbox`.
        """
        return Mailbox(self.addr, self.raft_node, self.chan)

    def get_peers(self) -> dict[int, RaftClient]:
        return self.raft_node.peers

    @staticmethod
    def set_cluster_config(config: RaftConfig) -> None:
        RaftCluster.cluster_config = config

    def is_initialized(self) -> bool:
        return self.raft_node is not None

    async def bootstrap_cluster(self) -> None:
        """
        Create a new leader for the cluster with node id 1.
        """
        self.raft_node = RaftNode.bootstrap_leader(
            self.chan,
            self.fsm,
            self.slog,
            self.logger,
            cluster_cfg=RaftCluster.cluster_config,
        )
        asyncio.create_task(RaftServer(self.addr, self.chan, self.logger).run())
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
            self.logger.info(f'Attempting to join the cluster through "{peer_addr}"...')

            leader_addr = None
            seek_next = False

            while not leader_addr:
                client = RaftClient(peer_addr)
                try:
                    resp = await client.request_id(raft_addr)
                except grpc.aio.AioRpcError:
                    seek_next = True
                    break

                match resp.result:
                    case raft_service_pb2.IdRequest_Success:
                        leader_addr = peer_addr
                        leader_id, node_id, peer_addrs = pickle.loads(resp.data)
                        break
                    case raft_service_pb2.IdRequest_WrongLeader:
                        _, peer_addr, _ = pickle.loads(resp.data)
                        self.logger.info(
                            f"Sent message to the wrong leader, retrying with the leader at {peer_addr}"
                        )
                        continue
                    case raft_service_pb2.IdRequest_Error | _:
                        raise UnknownError("Failed to join the cluster!")

            if not seek_next:
                break
        else:
            raise ClusterJoinError(
                "Could not join the cluster. Check your Raft configuration and check to make sure that any of them is alive."
            )

        assert leader_id is not None and node_id is not None

        self.logger.info(
            f"Cluster join succeeded. Obtained node id {node_id} from the leader node {leader_id}."
        )

        # 2. Run server and node to prepare for joining
        self.raft_node = RaftNode.new_follower(
            self.chan,
            node_id,
            self.fsm,
            self.slog,
            self.logger,
            cluster_cfg=RaftCluster.cluster_config,
        )

        self.raft_node.peers = {
            **{node_id: RaftClient(addr) for node_id, addr in peer_addrs.items()},
            leader_id: client,
        }

        asyncio.create_task(RaftServer(self.addr, self.chan, self.logger).run())
        raft_node_handle = asyncio.create_task(self.raft_node.run())

        # 3. Join the cluster
        conf_change = ConfChange.default()
        conf_change.set_node_id(node_id)
        conf_change.set_change_type(role.to_changetype())
        conf_change.set_context(pickle.dumps(self.addr))

        # TODO: Should handle wrong leader error here because the leader might change in the meanwhile.
        # But it might be already handled by the rerouting logic. So, it should be tested first.
        while True:
            resp = await client.change_config(conf_change)

            if resp.result == raft_service_pb2.ChangeConfig_Success:
                break
            elif resp.result == raft_service_pb2.ChangeConfig_TimeoutError:
                self.logger.info("Join request timeout. Retrying...")
                await asyncio.sleep(0.25)
                continue

        await raft_node_handle
