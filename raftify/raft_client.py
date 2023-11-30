import asyncio
import json
import math
from typing import Any, Optional

import grpc
from rraft import ConfChange, ConfChangeV2, Message

from .codec.abc import AbstractCodec
from .codec.pickle import PickleCodec
from .logger import AbstractRaftifyLogger
from .pb_adapter import ConfChangeV2Adapter, MessageAdapter
from .protos import eraftpb_pb2, raft_service_pb2, raft_service_pb2_grpc
from .raft_client_response import (
    ChangeConfigResponse,
    DebugEntriesResponse,
    DebugNodeResponse,
    GetPeersResponse,
    IdRequestResponse,
    ProposeResponse,
    VersionResponse,
)
from .utils import SocketAddr


class RaftClient:
    """
    Interface to communicate with the Raft Cluster.
    """

    def __init__(
        self,
        addr: str | SocketAddr,
        *,
        codec: AbstractCodec = PickleCodec(),
        grpc_connection_options: Optional[list[tuple[str, Any]]] = None,
        logger: Optional[AbstractRaftifyLogger] = None,
        credentials: Optional[grpc.ServerCredentials] = None,
    ):
        if isinstance(addr, str):
            addr = SocketAddr.from_str(addr)

        self.addr = addr
        self.credentials = credentials
        self.logger = logger
        self.codec = codec
        self.grpc_connection_options = grpc_connection_options
        self.grpc_channel = None
        self.first_failed_time: Optional[float] = None

    def __repr__(self) -> str:
        return f"RaftClient(addr={self.addr})"

    def to_dict(self) -> dict:
        return {
            "addr": str(self.addr),
            "credentials": self.credentials,
            "first_failed_time": self.first_failed_time,
        }

    async def __close_channel(self) -> None:
        await self.grpc_channel.close()
        self.grpc_channel = None

    async def __get_or_create_channel(self) -> grpc.aio.Channel:
        """
        Creates or reuses a gRPC channel.
        """

        if self.grpc_channel:
            try:
                if (
                    self.grpc_channel.get_state(try_to_connect=True)
                    != grpc.ChannelConnectivity.READY
                ):
                    await self.__close_channel()
            except Exception as e:
                if logger := self.logger:
                    logger.error(f"Connection reset by unknown error. Err: {e}")
                await self.__close_channel()

        if self.grpc_channel is None:
            if credentials := self.credentials:
                self.grpc_channel = grpc.aio.secure_channel(
                    str(self.addr), credentials, options=self.grpc_connection_options
                )
            else:
                self.grpc_channel = grpc.aio.insecure_channel(
                    str(self.addr), options=self.grpc_connection_options
                )

        return self.grpc_channel

    async def propose(self, data: bytes, *, timeout: float = 5.0) -> ProposeResponse:
        """
        Request to send a propose to the cluster.
        """

        request_args = raft_service_pb2.ProposeArgs(msg=data)
        stub = raft_service_pb2_grpc.RaftServiceStub(
            await self.__get_or_create_channel()
        )
        response: raft_service_pb2.ProposeResponse = await asyncio.wait_for(
            stub.Propose(request_args), timeout
        )

        return ProposeResponse(
            msg=self.codec.decode(response.msg),
            rejected=response.rejected,
        )

    async def change_config(
        self, conf_change: ConfChange | ConfChangeV2, *, timeout: float = math.inf
    ) -> ChangeConfigResponse:
        """
        Request for membership config change of the cluster.
        """
        if isinstance(conf_change, ConfChange):
            conf_change = conf_change.as_v2()

        request_args = ConfChangeV2Adapter.to_pb(conf_change)
        stub = raft_service_pb2_grpc.RaftServiceStub(
            await self.__get_or_create_channel()
        )
        response: raft_service_pb2.ChangeConfigResponse = await asyncio.wait_for(
            stub.ChangeConfig(request_args), timeout
        )

        return ChangeConfigResponse(
            result=str(response.result), data=self.codec.decode(response.data)
        )

    async def apply_change_config_forcely(
        self, conf_change: ConfChange | ConfChangeV2, *, timeout: float = math.inf
    ) -> ChangeConfigResponse:
        """
        Request for membership config change of the cluster.
        Note that this method is not safe and even not verified.
        """
        if isinstance(conf_change, ConfChange):
            conf_change = conf_change.as_v2()

        request_args = ConfChangeV2Adapter.to_pb(conf_change)
        stub = raft_service_pb2_grpc.RaftServiceStub(
            await self.__get_or_create_channel()
        )
        response: raft_service_pb2.ChangeConfigResponse = await asyncio.wait_for(
            stub.ApplyConfigChangeForcely(request_args), timeout
        )

        return ChangeConfigResponse(
            result=str(response.result), data=self.codec.decode(response.data)
        )

    async def send_message(self, msg: Message, *, timeout: float = 5.0) -> None:
        """
        Low level API to send a Raft Message to the cluster.
        If you are not certain about what this function does, do not use it.
        """

        request_args = MessageAdapter.to_pb(msg)
        stub = raft_service_pb2_grpc.RaftServiceStub(
            await self.__get_or_create_channel()
        )

        _: raft_service_pb2.Empty = await asyncio.wait_for(
            stub.SendMessage(request_args), timeout
        )
        return None

    async def request_id(
        self, addr: SocketAddr, *, timeout: float = 5.0
    ) -> IdRequestResponse:
        """
        Request to reserve an ID for a new node to join the cluster.
        """

        request_args = raft_service_pb2.IdRequestArgs(addr=str(addr))
        stub = raft_service_pb2_grpc.RaftServiceStub(
            await self.__get_or_create_channel()
        )
        response: raft_service_pb2.IdRequestResponse = await asyncio.wait_for(
            stub.RequestId(request_args), timeout
        )
        return IdRequestResponse(
            result=str(response.result),
            leader_id=response.leader_id,
            leader_addr=response.leader_addr,
            reserved_id=response.reserved_id,
            peers=self.codec.decode(response.peers),
        )

    async def member_bootstrap_ready(
        self, follower_id: int, *, timeout: float = 5.0
    ) -> None:
        """
        Request to notify that a follower is ready to bootstrap.
        Made from follower to leader.
        Used in the initial cluster bootstrap.
        """

        request_args = raft_service_pb2.MemberBootstrapReadyArgs(
            follower_id=follower_id
        )
        stub = raft_service_pb2_grpc.RaftServiceStub(
            await self.__get_or_create_channel()
        )
        _: raft_service_pb2.MemberBootstrapReadyResponse = await asyncio.wait_for(
            stub.MemberBootstrapReady(request_args), timeout
        )
        return None

    async def cluster_bootstrap_ready(
        self, peers: bytes, *, timeout: float = 5.0
    ) -> None:
        """
        Request to notify that a leader is ready to bootstrap the cluster.
        Made from leader to follower.
        Used in the initial cluster bootstrap.
        """

        request_args = raft_service_pb2.ClusterBootstrapReadyArgs(peers=peers)
        stub = raft_service_pb2_grpc.RaftServiceStub(
            await self.__get_or_create_channel()
        )
        _: raft_service_pb2.ClusterBootstrapReadyResponse = await asyncio.wait_for(
            stub.ClusterBootstrapReady(request_args), timeout
        )
        return None

    async def reroute_message(
        self,
        reroute_msg_type: raft_service_pb2.RerouteMsgType,
        msg_bytes: Optional[bytes] = None,
        conf_change: Optional[eraftpb_pb2.ConfChangeV2] = None,
        *,
        timeout: float = 5.0,
    ) -> ProposeResponse:
        """
        Request to reroute a message to the leader.
        """

        request_args = raft_service_pb2.RerouteMessageArgs(
            proposed_data=msg_bytes or b"",
            conf_change=conf_change,
            type=reroute_msg_type,
        )
        stub = raft_service_pb2_grpc.RaftServiceStub(
            await self.__get_or_create_channel()
        )
        response: raft_service_pb2.ProposeResponse = await asyncio.wait_for(
            stub.RerouteMessage(request_args), timeout
        )

        return ProposeResponse(
            msg=self.codec.decode(response.msg),
            rejected=response.rejected,
        )

    async def debug_node(self, *, timeout: float = 5.0) -> DebugNodeResponse:
        """
        Request to debug the node.
        """

        request_args = raft_service_pb2.Empty()
        stub = raft_service_pb2_grpc.RaftServiceStub(
            await self.__get_or_create_channel()
        )
        response: raft_service_pb2.DebugNodeResponse = await asyncio.wait_for(
            stub.DebugNode(request_args), timeout
        )
        return DebugNodeResponse(result=json.loads(str(response.result)))

    async def debug_entries(self, *, timeout: float = 5.0) -> DebugEntriesResponse:
        """
        Request to debug the node.
        """

        request_args = raft_service_pb2.Empty()
        stub = raft_service_pb2_grpc.RaftServiceStub(
            await self.__get_or_create_channel()
        )
        response: raft_service_pb2.DebugEntriesResponse = await asyncio.wait_for(
            stub.DebugEntries(request_args), timeout
        )
        return DebugEntriesResponse(result=json.loads(str(response.result)))

    async def version(self, *, timeout: float = 5.0) -> VersionResponse:
        """
        Request to get RaftServer's raftify version.
        """

        request_args = raft_service_pb2.Empty()
        stub = raft_service_pb2_grpc.RaftServiceStub(
            await self.__get_or_create_channel()
        )
        response: raft_service_pb2.VersionResponse = await asyncio.wait_for(
            stub.Version(request_args), timeout
        )
        return VersionResponse(result=str(response.result))

    async def get_peers(self, *, timeout: float = 5.0) -> GetPeersResponse:
        """ """

        request_args = raft_service_pb2.Empty()
        stub = raft_service_pb2_grpc.RaftServiceStub(
            await self.__get_or_create_channel()
        )
        response: raft_service_pb2.GetPeersResponse = await asyncio.wait_for(
            stub.GetPeers(request_args), timeout
        )
        return GetPeersResponse(peers=self.codec.decode(response.peers))
