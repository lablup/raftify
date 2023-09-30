import asyncio
import pickle
from asyncio import Queue
from dataclasses import dataclass
from typing import Tuple

import grpc
from rraft import ConfChangeSingle, ConfChangeType, ConfChangeV2, Logger, LoggerRef

from raftify.config import RaftifyConfig
from raftify.error import ClusterJoinError, LeaderNotFoundError, UnknownError
from raftify.follower_role import FollowerRole
from raftify.fsm import FSM
from raftify.logger import AbstractRaftifyLogger
from raftify.mailbox import Mailbox
from raftify.pb_adapter import ConfChangeV2Adapter
from raftify.peers import Peers
from raftify.protos import raft_service_pb2
from raftify.raft_client import RaftClient
from raftify.raft_node import RaftNode
from raftify.raft_server import RaftServer
from raftify.request_message import ConfigChangeReqMessage
from raftify.response_message import JoinSuccessRespMessage
from raftify.utils import SocketAddr


@dataclass
class RequestIdResponse:
    follower_id: int
    leader: Tuple[int, RaftClient]  # (leader_id, leader_client)
    peer_addrs: dict[int, SocketAddr]


class FollowerInfo:
    node_id: int
    addr: SocketAddr
    role: FollowerRole


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
        self.cluster_config = cluster_config
        self.peers = Peers({})
        self.raft_node = None
        self.raft_server = None
        self.raft_node_task = None
        self.raft_server_task = None

    def __ensure_initialized(self) -> None:
        assert self.raft_node and self.raft_server, "The raft node is not initialized!"

    def is_initialized(self) -> bool:
        return self.raft_node is not None and self.raft_server is not None

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
    ) -> int:
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

                        # self.peers = Peers(
                        #     {
                        #         **{
                        #             node_id: RaftClient(addr)
                        #             for node_id, addr in peer_addrs.items()
                        #         },
                        #         leader_id: RaftClient(leader_addr),
                        #     }
                        # )

                        # if node_id in self.peers.keys():
                        # del self.peers[node_id]

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

        return node_id

    async def join_followers(
        self,
    ) -> None:
        """ """
        self.__ensure_initialized()
        assert self.raft_node.is_leader(), "Only leader can join followers!"

        conf_change_v2 = ConfChangeV2.default()
        changes = []
        addrs = []

        for node_id in self.peers.keys():
            conf_change = ConfChangeSingle.default()
            conf_change.set_node_id(node_id)
            conf_change.set_change_type(ConfChangeType.AddNode)
            changes.append(conf_change)
            addrs.append(str(self.peers[node_id].addr))

        conf_change_v2.set_changes(changes)
        conf_change_v2.set_context(pickle.dumps(addrs))

        try:
            receiver = Queue()
            await self.raft_node.chan.put(
                ConfigChangeReqMessage(
                    ConfChangeV2Adapter.to_pb(conf_change_v2), receiver
                )
            )
            resp = await asyncio.wait_for(receiver.get(), 2)

        except grpc.aio.AioRpcError as e:
            raise ClusterJoinError(cause=e)

        except Exception as e:
            raise ClusterJoinError(cause=e)

        if isinstance(resp, JoinSuccessRespMessage):
            self.logger.info("All follower nodes successfully joined the cluster.")
            return
        # TODO: handle error cases

    async def run_raft(self, node_id: int) -> None:
        """ """
        self.logger.info(
            "Start to run RaftNode. Raftify config: " + str(self.cluster_config)
        )
        self.raft_server = RaftServer(self.addr, self.chan, self.logger)

        if node_id == 1:
            self.raft_node = RaftNode.bootstrap_leader(
                chan=self.chan,
                fsm=self.fsm,
                raft_server=self.raft_server,
                peers=self.peers,
                slog=self.slog,
                logger=self.logger,
                raftify_cfg=self.cluster_config,
            )
        else:
            self.raft_node = RaftNode.new_follower(
                chan=self.chan,
                id=node_id,
                fsm=self.fsm,
                raft_server=self.raft_server,
                peers=self.peers,
                slog=self.slog,
                logger=self.logger,
                raftify_cfg=self.cluster_config,
            )

        self.raft_server_task = asyncio.create_task(self.raft_server.run())
        self.raft_node_task = asyncio.create_task(self.raft_node.run())

        try:
            await asyncio.gather(*(self.raft_server_task, self.raft_node_task))
        except asyncio.CancelledError:
            self.logger.info("Raft server is cancelled. preparing to terminate...")
            await self.raft_server.terminate()
