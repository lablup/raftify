import asyncio
import pickle
from asyncio import Queue
from dataclasses import dataclass
from typing import Tuple

import grpc
from rraft import (
    ConfChangeSingle,
    ConfChangeTransition,
    ConfChangeType,
    ConfChangeV2,
    Logger,
    LoggerRef,
)

from raftify.config import RaftifyConfig
from raftify.error import ClusterBootstrapError, LeaderNotFoundError, UnknownError
from raftify.follower_role import FollowerRole
from raftify.fsm import FSM
from raftify.logger import AbstractRaftifyLogger
from raftify.mailbox import Mailbox
from raftify.pb_adapter import ConfChangeV2Adapter
from raftify.peers import Peer, Peers, PeerState
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
    peers: Peers


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
        peers: Peers,
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
        self.peers = peers
        self.raft_node = None
        self.raft_server = None
        self.raft_node_task = None
        self.raft_server_task = None

        if self_peer_id := self.peers.get_node_id_by_addr(self.addr):
            self.peers.connect(self_peer_id, self.addr)

    def __ensure_initialized(self) -> None:
        assert self.raft_node and self.raft_server, "The raft node is not initialized!"

    def is_initialized(self) -> bool:
        return self.raft_node is not None and self.raft_server is not None

    def cluster_bootstrap_ready(self, expected_size: int) -> bool:
        return len(self.peers) >= expected_size and all(
            data.state == PeerState.Connected for data in self.peers.data.values()
        )

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
        Used only in the dynamic cluster join process.
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
                        leader_id, node_id, raw_peers = pickle.loads(resp.data)
                        peers = Peers.decode(raw_peers)

                        self.peers = peers
                        self.peers.data[leader_id] = Peer(
                            addr=leader_addr, client=RaftClient(leader_addr)
                        )

                        # if node_id in self.peers.data.keys():
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
            f"Obtained node id {node_id} successfully from the leader node {leader_id}."
        )

        return node_id

    async def join_followers(
        self,
    ) -> None:
        """ """
        self.__ensure_initialized()
        assert self.raft_node.is_leader(), "Only leader can join followers!"

        conf_change_v2 = ConfChangeV2.default()
        conf_change_v2.set_transition(ConfChangeTransition.Explicit)
        changes = []
        addrs = []

        for node_id in self.peers.data.keys():
            # Skip leader
            if self.addr == self.peers.data[node_id].addr:
                continue

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
            raise ClusterBootstrapError(cause=e)

        except Exception as e:
            raise ClusterBootstrapError(cause=e)

        if isinstance(resp, JoinSuccessRespMessage):
            self.logger.info("All follower nodes successfully joined the cluster.")
            return
        # TODO: handle error cases

    def run_raft(self, node_id: int) -> None:
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

    async def wait_for_termination(self) -> None:
        """ """
        self.__ensure_initialized()

        try:
            await asyncio.gather(*(self.raft_server_task, self.raft_node_task))
        except asyncio.CancelledError:
            self.logger.info("Raft server is cancelled. preparing to terminate...")
            await self.raft_server.terminate()
