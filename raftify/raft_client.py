import asyncio
from typing import Optional

import grpc
from rraft import ConfChange, Message

from raftify.pb_adapter import ConfChangeAdapter, MessageAdapter
from raftify.protos import raft_service_pb2, raft_service_pb2_grpc
from raftify.utils import SocketAddr


class RaftClient:
    def __init__(
        self, addr: SocketAddr, *, credentials: Optional[grpc.ServerCredentials] = None
    ):
        self.addr = addr
        self.credentials = credentials

    def __repr__(self) -> str:
        return f"RaftClient(addr={self.addr})"

    def __create_channel(self) -> grpc.aio.Channel:
        if credentials := self.credentials:
            return grpc.aio.secure_channel(str(self.addr), credentials)
        return grpc.aio.insecure_channel(str(self.addr))

    async def change_config(
        self, cc: ConfChange, timeout: float = 5.0
    ) -> raft_service_pb2.ChangeConfigResponse:
        request = ConfChangeAdapter.to_pb(cc)

        async with self.__create_channel() as channel:
            stub = raft_service_pb2_grpc.RaftServiceStub(channel)
            return await asyncio.wait_for(stub.ChangeConfig(request), timeout)

    async def send_message(
        self, msg: Message, timeout: float = 5.0
    ) -> raft_service_pb2.RaftMessageResponse:
        request = MessageAdapter.to_pb(msg)

        async with self.__create_channel() as channel:
            stub = raft_service_pb2_grpc.RaftServiceStub(channel)
            return await asyncio.wait_for(stub.SendMessage(request), timeout)

    async def request_id(
        self, addr: SocketAddr, timeout: float = 5.0
    ) -> raft_service_pb2.IdRequestResponse:
        request = raft_service_pb2.IdRequestArgs(addr=str(addr))

        async with self.__create_channel() as channel:
            stub = raft_service_pb2_grpc.RaftServiceStub(channel)
            return await asyncio.wait_for(stub.RequestId(request), timeout)

    async def reroute_message(
        self,
        reroute_msg_type: raft_service_pb2.RerouteMsgType,
        msg_bytes: Optional[bytes] = None,
        confchange: Optional[raft_service_pb2.ConfChange] = None,
        timeout: float = 5.0,
    ) -> raft_service_pb2.RaftMessageResponse:
        request = raft_service_pb2.RerouteMessageArgs(
            proposed_data=msg_bytes or b"",
            conf_change=confchange,
            type=reroute_msg_type,
        )

        async with self.__create_channel() as channel:
            stub = raft_service_pb2_grpc.RaftServiceStub(channel)
            return await asyncio.wait_for(stub.RerouteMessage(request), timeout)
