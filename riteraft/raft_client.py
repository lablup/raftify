import asyncio
from typing import Optional

import grpc
from rraft import ConfChange, Message

from riteraft.protos import eraftpb_pb2, raft_service_pb2, raft_service_pb2_grpc
from riteraft.utils import SocketAddr


class RaftClient:
    def __init__(
        self, addr: SocketAddr, *, credentials: Optional[grpc.ServerCredentials] = None
    ):
        self.addr = addr
        self.credentials = credentials

    def __create_channel(self) -> grpc.aio.Channel:
        if credentials := self.credentials:
            return grpc.aio.secure_channel(self.addr, credentials)
        return grpc.aio.insecure_channel(self.addr)

    async def change_config(
        self, cc: ConfChange, timeout: float = 5.0
    ) -> raft_service_pb2.RaftResponse:
        request = eraftpb_pb2.ConfChange(
            change_type=cc.get_change_type(),
            context=cc.get_context(),
            id=cc.get_id(),
            node_id=cc.get_node_id(),
        )

        async with self.__create_channel() as channel:
            stub = raft_service_pb2_grpc.RaftServiceStub(channel)
            return await asyncio.wait_for(stub.ChangeConfig(request), timeout)

    async def send_message(
        self, msg: Message, timeout: float = 5.0
    ) -> raft_service_pb2.RaftResponse:
        request = eraftpb_pb2.Message(
            commit=msg.get_commit(),
            context=msg.get_context(),
            entries=msg.get_entries(),
            index=msg.get_index(),
            log_term=msg.get_log_term(),
            msg_type=msg.get_msg_type(),
            priority=msg.get_priority(),
            reject_hint=msg.get_reject_hint(),
            reject=msg.get_reject(),
            request_snapshot=msg.get_request_snapshot(),
            snapshot=msg.get_snapshot(),
            term=msg.get_term(),
            to=msg.get_to(),
        )

        async with self.__create_channel() as channel:
            stub = raft_service_pb2_grpc.RaftServiceStub(channel)
            return await asyncio.wait_for(stub.SendMessage(request), timeout)

    async def request_id(
        self, timeout: float = 5.0
    ) -> raft_service_pb2.IdRequestResponse:
        request = raft_service_pb2.Empty()

        async with self.__create_channel() as channel:
            stub = raft_service_pb2_grpc.RaftServiceStub(channel)
            return await asyncio.wait_for(stub.RequestId(request), timeout)
