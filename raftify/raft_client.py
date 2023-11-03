import asyncio
import math
from typing import Optional

import grpc
from rraft import ConfChangeV2, Message

from .pb_adapter import ConfChangeV2Adapter, MessageAdapter
from .protos import eraftpb_pb2, raft_service_pb2, raft_service_pb2_grpc
from .utils import SocketAddr


class RaftClient:
    """
    Low level interface to communicate with the RaftFacade.
    """

    def __init__(
        self, addr: SocketAddr, *, credentials: Optional[grpc.ServerCredentials] = None
    ):
        self.addr = addr
        self.credentials = credentials
        self.first_failed_time: Optional[float] = None

    def __repr__(self) -> str:
        return f"RaftClient(addr={self.addr})"

    def to_dict(self) -> dict:
        return {
            "addr": str(self.addr),
            "credentials": self.credentials,
            "first_failed_time": self.first_failed_time,
        }

    def __create_channel(self) -> grpc.aio.Channel:
        if credentials := self.credentials:
            return grpc.aio.secure_channel(str(self.addr), credentials)
        return grpc.aio.insecure_channel(str(self.addr))

    async def change_config(
        self, conf_change: ConfChangeV2, timeout: float = math.inf
    ) -> raft_service_pb2.ChangeConfigResponse:
        """
        Request to membership config change of the cluster.
        """

        request = ConfChangeV2Adapter.to_pb(conf_change)

        async with self.__create_channel() as channel:
            stub = raft_service_pb2_grpc.RaftServiceStub(channel)
            return await asyncio.wait_for(stub.ChangeConfig(request), timeout)

    async def apply_change_config_forcely(
        self, conf_change: ConfChangeV2, timeout: float = math.inf
    ) -> raft_service_pb2.ChangeConfigResponse:
        """
        Request to membership config change of the cluster and apply it forcely.
        Note that this method is not safe and even not verified.
        """

        request = ConfChangeV2Adapter.to_pb(conf_change)

        async with self.__create_channel() as channel:
            stub = raft_service_pb2_grpc.RaftServiceStub(channel)
            return await asyncio.wait_for(
                stub.ApplyConfigChangeForcely(request), timeout
            )

    async def send_message(
        self, msg: Message, timeout: float
    ) -> raft_service_pb2.SendMessageResponse:
        """
        Request to send a message to the cluster.
        """

        request = MessageAdapter.to_pb(msg)

        async with self.__create_channel() as channel:
            stub = raft_service_pb2_grpc.RaftServiceStub(channel)
            return await asyncio.wait_for(stub.SendMessage(request), timeout)

    async def propose(
        self, data: bytes, timeout: float
    ) -> raft_service_pb2.ProposeResponse:
        """
        Request to send a propose to the cluster.
        """

        async with self.__create_channel() as channel:
            stub = raft_service_pb2_grpc.RaftServiceStub(channel)
            return await asyncio.wait_for(stub.Propose(data), timeout)

    async def request_id(
        self, addr: SocketAddr, timeout: float
    ) -> raft_service_pb2.IdRequestResponse:
        """
        Request to reserve an ID for a new node to join the cluster.
        """

        request_args = raft_service_pb2.IdRequestArgs(addr=str(addr))

        async with self.__create_channel() as channel:
            stub = raft_service_pb2_grpc.RaftServiceStub(channel)
            return await asyncio.wait_for(stub.RequestId(request_args), timeout)

    async def member_bootstrap_ready(
        self, follower_id: int, timeout: float
    ) -> raft_service_pb2.MemberBootstrapReadyResponse:
        """
        Request to notify that a follower is ready to bootstrap.
        From follower to leader.
        Used in initial cluster bootstrap.
        """

        request_args = raft_service_pb2.MemberBootstrapReadyArgs(
            follower_id=follower_id
        )

        async with self.__create_channel() as channel:
            stub = raft_service_pb2_grpc.RaftServiceStub(channel)
            return await asyncio.wait_for(
                stub.MemberBootstrapReady(request_args), timeout
            )

    async def cluster_bootstrap_ready(
        self, peers: bytes, timeout: float
    ) -> raft_service_pb2.ClusterBootstrapReadyResponse:
        """
        Request to notify that a leader is ready to bootstrap the cluster.
        From leader to follower.
        Used in initial cluster bootstrap.
        """

        request_args = raft_service_pb2.ClusterBootstrapReadyArgs(peers=peers)

        async with self.__create_channel() as channel:
            stub = raft_service_pb2_grpc.RaftServiceStub(channel)
            return await asyncio.wait_for(
                stub.ClusterBootstrapReady(request_args), timeout
            )

    async def reroute_message(
        self,
        reroute_msg_type: raft_service_pb2.RerouteMsgType,
        timeout: float,
        msg_bytes: Optional[bytes] = None,
        conf_change: Optional[eraftpb_pb2.ConfChangeV2] = None,
    ) -> raft_service_pb2.SendMessageResponse:
        """
        Request to reroute a message to the leader.
        """

        request = raft_service_pb2.RerouteMessageArgs(
            proposed_data=msg_bytes or b"",
            conf_change=conf_change,
            type=reroute_msg_type,
        )

        async with self.__create_channel() as channel:
            stub = raft_service_pb2_grpc.RaftServiceStub(channel)
            return await asyncio.wait_for(stub.RerouteMessage(request), timeout)

    async def debug_node(self, timeout: float) -> raft_service_pb2.DebugNodeResponse:
        """
        Request to debug the node.
        """

        request = raft_service_pb2.Empty()

        async with self.__create_channel() as channel:
            stub = raft_service_pb2_grpc.RaftServiceStub(channel)
            return await asyncio.wait_for(stub.DebugNode(request), timeout)

    async def debug_entries(
        self, timeout: float
    ) -> raft_service_pb2.DebugEntriesResponse:
        """
        Request to debug the node.
        """

        request = raft_service_pb2.Empty()

        async with self.__create_channel() as channel:
            stub = raft_service_pb2_grpc.RaftServiceStub(channel)
            return await asyncio.wait_for(stub.DebugEntries(request), timeout)

    async def version(self, timeout: float) -> raft_service_pb2.VersionResponse:
        """
        Request to debug raftify version.
        """

        request = raft_service_pb2.Empty()

        async with self.__create_channel() as channel:
            stub = raft_service_pb2_grpc.RaftServiceStub(channel)
            return await asyncio.wait_for(stub.Version(request), timeout)
