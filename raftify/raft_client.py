import asyncio
from typing import Optional

import grpc
from rraft import ConfChangeV2, Message

from raftify.pb_adapter import ConfChangeV2Adapter, MessageAdapter
from raftify.protos import eraftpb_pb2, raft_service_pb2, raft_service_pb2_grpc
from raftify.utils import AtomicInteger, SocketAddr


class RaftClient:
    def __init__(
        self, addr: SocketAddr, *, credentials: Optional[grpc.ServerCredentials] = None
    ):
        self.addr = addr
        self.credentials = credentials
        self.failed_request_counter = AtomicInteger()

    def __repr__(self) -> str:
        return f"RaftClient(addr={self.addr})"

    def to_dict(self) -> dict:
        return {
            "addr": str(self.addr),
            "credentials": self.credentials,
            "failed_request_counter": self.failed_request_counter.value,
        }

    def __create_channel(self) -> grpc.aio.Channel:
        if credentials := self.credentials:
            return grpc.aio.secure_channel(str(self.addr), credentials)
        return grpc.aio.insecure_channel(str(self.addr))

    async def change_config(
        self, conf_change: ConfChangeV2, timeout: float
    ) -> raft_service_pb2.ChangeConfigResponse:
        request = ConfChangeV2Adapter.to_pb(conf_change)

        async with self.__create_channel() as channel:
            stub = raft_service_pb2_grpc.RaftServiceStub(channel)
            return await asyncio.wait_for(stub.ChangeConfig(request), timeout)

    async def send_message(
        self, msg: Message, timeout: float
    ) -> raft_service_pb2.RaftMessageResponse:
        request = MessageAdapter.to_pb(msg)

        async with self.__create_channel() as channel:
            stub = raft_service_pb2_grpc.RaftServiceStub(channel)
            return await asyncio.wait_for(stub.SendMessage(request), timeout)

    async def request_id(
        self, addr: SocketAddr, timeout: float
    ) -> raft_service_pb2.IdRequestResponse:
        request = raft_service_pb2.IdRequestArgs(addr=str(addr))

        async with self.__create_channel() as channel:
            stub = raft_service_pb2_grpc.RaftServiceStub(channel)
            return await asyncio.wait_for(stub.RequestId(request), timeout)

    async def member_bootstrap_ready(
        self, follower_id: int, timeout: float
    ) -> raft_service_pb2.RaftMessageResponse:
        request = raft_service_pb2.MemberBootstrapReadyArgs(follower_id=follower_id)

        async with self.__create_channel() as channel:
            stub = raft_service_pb2_grpc.RaftServiceStub(channel)
            return await asyncio.wait_for(stub.MemberBootstrapReady(request), timeout)

    async def cluster_bootstrap_ready(
        self, peers: bytes, timeout: float
    ) -> raft_service_pb2.RaftMessageResponse:
        request = raft_service_pb2.ClusterBootstrapReadyArgs(peers=peers)

        async with self.__create_channel() as channel:
            stub = raft_service_pb2_grpc.RaftServiceStub(channel)
            return await asyncio.wait_for(stub.ClusterBootstrapReady(request), timeout)

    async def reroute_message(
        self,
        reroute_msg_type: raft_service_pb2.RerouteMsgType,
        timeout: float,
        msg_bytes: Optional[bytes] = None,
        conf_change: Optional[eraftpb_pb2.ConfChangeV2] = None,
    ) -> raft_service_pb2.RaftMessageResponse:
        request = raft_service_pb2.RerouteMessageArgs(
            proposed_data=msg_bytes or b"",
            conf_change=conf_change,
            type=reroute_msg_type,
        )

        async with self.__create_channel() as channel:
            stub = raft_service_pb2_grpc.RaftServiceStub(channel)
            return await asyncio.wait_for(stub.RerouteMessage(request), timeout)
