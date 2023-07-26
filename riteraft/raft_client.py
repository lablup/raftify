import asyncio
from typing import Optional

import grpc
from rraft import ConfChange, Message

from riteraft.pb_adapter import ConfChangeAdapter, MessageAdapter
from riteraft.protos import raft_service_pb2, raft_service_pb2_grpc
from riteraft.utils import SocketAddr


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
    ) -> raft_service_pb2.RaftResponse:
        request = ConfChangeAdapter.to_pb(cc)

        async with self.__create_channel() as channel:
            stub = raft_service_pb2_grpc.RaftServiceStub(channel)
            return await asyncio.wait_for(stub.ChangeConfig(request), timeout)

    async def send_message(
        self, msg: Message, timeout: float = 5.0
    ) -> raft_service_pb2.RaftResponse:
        request = MessageAdapter.to_pb(msg)

        async with self.__create_channel() as channel:
            stub = raft_service_pb2_grpc.RaftServiceStub(channel)
            return await asyncio.wait_for(stub.SendMessage(request), timeout)

    async def request_id(
        self, addr: SocketAddr, timeout: float = 5.0
    ) -> raft_service_pb2.IdRequestResponse:
        request = raft_service_pb2.RequestIdArgs(addr=str(addr))

        async with self.__create_channel() as channel:
            stub = raft_service_pb2_grpc.RaftServiceStub(channel)
            return await asyncio.wait_for(stub.RequestId(request), timeout)
