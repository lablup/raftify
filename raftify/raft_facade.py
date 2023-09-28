import asyncio
import pickle
from asyncio import Queue
from dataclasses import dataclass
from enum import Enum
from typing import Optional, Tuple

import grpc
from rraft import ConfChange, Logger, LoggerRef

from raftify.config import RaftifyConfig
from raftify.error import ClusterJoinError, LeaderNotFoundError, UnknownError
from raftify.follower_role import FollowerRole
from raftify.fsm import FSM
from raftify.logger import AbstractRaftifyLogger
from raftify.mailbox import Mailbox
from raftify.peers import Peers
from raftify.protos import raft_service_pb2
from raftify.raft_client import RaftClient
from raftify.raft_node import RaftNode
from raftify.raft_server import RaftServer
from raftify.utils import SocketAddr


class RaftNodeRole(Enum):
    Leader = 0
    Follower = 1


@dataclass
class RequestIdResponse:
    follower_id: int
    leader: Tuple[int, RaftClient]  # (leader_id, leader_client)
    peer_addrs: dict[int, SocketAddr]


class RaftCluster:
    def __init__(
        self,
        cluster_config: RaftifyConfig,
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
        self.chan: Queue = Queue(maxsize=100)
        self.raft_node = None
        self.raft_server = None
        self.raft_node_task = None
        self.cluster_config = cluster_config
        self.raft_server_task = None

    def __ensure_initialized(self) -> None:
        assert self.raft_node and self.raft_server, "The raft node is not initialized!"

    def __ensure_task_running(self) -> None:
        assert (
            self.raft_server_task
        ), "Raft server is not running! Call `bootstrap_cluster` or `join_cluster` first."
        assert (
            self.raft_node_task
        ), "Raft node is not running! Call `bootstrap_cluster` or `join_cluster` first."

    @property
    def mailbox(self) -> Mailbox:
        """
        Get the node's `Mailbox`.
        """
        self.__ensure_initialized()
        return Mailbox(self.addr, self.raft_node, self.chan, self.cluster_config)

    def get_peers(self) -> Peers:
        self.__ensure_initialized()
        return self.raft_node.peers

    def is_initialized(self) -> bool:
        return self.raft_node is not None

    def build_raft(self, role: RaftNodeRole, follower_id: Optional[int] = None) -> None:
        """
        Prepare Raft node and Raft server with the given role before using it.
        It should be called before `bootstrap_cluster` or `join_cluster`.
        """
        self.raft_server = RaftServer(self.addr, self.chan, self.logger)
        assert self.raft_server is not None
        self.logger.info("Raftify config: " + str(self.cluster_config))

        if role == RaftNodeRole.Follower:
            assert follower_id is not None

            self.raft_node = RaftNode.new_follower(
                chan=self.chan,
                id=follower_id,
                fsm=self.fsm,
                raft_server=self.raft_server,
                slog=self.slog,
                logger=self.logger,
                raftify_cfg=self.cluster_config,
            )
        else:
            self.raft_node = RaftNode.bootstrap_leader(
                chan=self.chan,
                fsm=self.fsm,
                raft_server=self.raft_server,
                slog=self.slog,
                logger=self.logger,
                raftify_cfg=self.cluster_config,
            )

    def bootstrap_cluster(self) -> None:
        """
        Create a new leader for the cluster with node_id 1.
        """
        self.__ensure_initialized()
        self.raft_server_task = asyncio.create_task(self.raft_server.run())
        self.raft_node_task = asyncio.create_task(self.raft_node.run())

    async def merge_cluster(
        self,
        leader_node_id: int,
        raft_addr: SocketAddr,
        peer_candidates: list[SocketAddr],
    ) -> None:
        self.__ensure_initialized()

        if self.raft_node.is_leader():
            print("Step down from the leader...")
            term = self.raft_node.raw_node.get_raft().get_term()
            self.raft_node.raw_node.get_raft().become_follower(term, leader_node_id)

        resp = await self.request_id(raft_addr, peer_candidates)
        await self.join_cluster(resp)

    async def create_snapshot(self) -> None:
        self.__ensure_initialized()

        hs = self.raft_node.lmdb.core.hard_state()
        await self.raft_node.create_snapshot(
            self.raft_node.lmdb.last_index(), hs.get_term()
        )

    def transfer_leader(
        self,
        node_id: int,
    ) -> bool:
        self.__ensure_initialized()

        if not self.raft_node.is_leader():
            self.logger.warning("LeaderTransfer requested but not leader!")
            return False

        self.raft_node.raw_node.transfer_leader(node_id)
        return True

    async def request_id(
        self, raft_addr: SocketAddr, peer_candidates: list[SocketAddr]
    ) -> RequestIdResponse:
        """
        To join the cluster, find out which node is the leader and get node_id from the leader.
        """

        for peer_addr in peer_candidates:
            self.logger.info(f'Attempting to join the cluster through "{peer_addr}"...')

            leader_addr = None
            seek_next = False

            while not leader_addr:
                client = RaftClient(peer_addr)
                try:
                    resp = await client.request_id(
                        raft_addr, timeout=self.cluster_config.message_timeout
                    )
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
                            f"Sent message to the wrong leader, retrying with the leader at {peer_addr}."
                        )
                        continue
                    case raft_service_pb2.IdRequest_Error | _:
                        raise UnknownError("Failed to join the cluster!")

            if not seek_next:
                break
        else:
            raise LeaderNotFoundError()

        assert leader_id is not None and node_id is not None

        self.logger.info(
            f"Obtained node id {node_id} from the leader node {leader_id}."
        )

        return RequestIdResponse(node_id, (leader_id, client), peer_addrs)

    async def join_cluster(
        self,
        request_id_response: RequestIdResponse,
        role: FollowerRole = FollowerRole.Voter,
    ) -> None:
        """
        Try to join a new cluster through `peer_candidates` and get `node id` from the cluster's leader.
        """
        self.__ensure_initialized()

        node_id = request_id_response.follower_id
        leader = request_id_response.leader
        peer_addrs = request_id_response.peer_addrs
        leader_id, leader_client = leader

        self.raft_node.peers = Peers(
            {
                **{node_id: RaftClient(addr) for node_id, addr in peer_addrs.items()},
                leader_id: leader_client,
            }
        )

        self.raft_server_task = asyncio.create_task(self.raft_server.run())
        self.raft_node_task = asyncio.create_task(self.raft_node.run())

        conf_change = ConfChange.default()
        conf_change.set_node_id(node_id)
        conf_change.set_change_type(role.to_changetype())
        conf_change.set_context(pickle.dumps(self.addr))

        conf_change_v2 = conf_change.as_v2()

        # TODO: Should handle wrong leader error here because the leader might change in the meanwhile.
        # But it might be already handled by the rerouting logic. So, it should be tested first.
        while True:
            try:
                resp = await leader_client.change_config(
                    conf_change_v2, timeout=self.cluster_config.message_timeout
                )

            except grpc.aio.AioRpcError as e:
                raise ClusterJoinError(cause=e)

            except Exception as e:
                raise ClusterJoinError(cause=e)

            if resp.result == raft_service_pb2.ChangeConfig_Success:
                return
            elif resp.result == raft_service_pb2.ChangeConfig_TimeoutError:
                self.logger.info("Join request timeout. Retrying...")
                await asyncio.sleep(2)
                continue

    async def run_raft(self) -> None:
        """
        Start to run the raft node.
        """
        self.__ensure_initialized()
        self.__ensure_task_running()

        try:
            await asyncio.gather(*(self.raft_server_task, self.raft_node_task))
        except asyncio.CancelledError:
            self.logger.info("Raft server is cancelled. preparing to terminate...")
            await self.raft_server.terminate()
