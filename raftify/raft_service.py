import asyncio
import logging
import pickle
from asyncio import Queue

import grpc

from raftify.protos import eraftpb_pb2, raft_service_pb2
from raftify.request_message import (
    MessageConfigChange,
    MessageRaft,
    MessageRequestId,
    MessageRerouteToLeader,
)
from raftify.response_message import (
    RaftRespError,
    RaftRespIdReserved,
    RaftRespJoinSuccess,
    RaftRespOk,
    RaftRespResponse,
    RaftRespWrongLeader,
)


class RaftService:
    def __init__(self, sender: Queue) -> None:
        self.sender = sender

    async def RequestId(
        self, request: raft_service_pb2.RequestIdArgs, context: grpc.aio.ServicerContext
    ) -> raft_service_pb2.IdRequestResponse:
        receiver = Queue()
        try:
            await self.sender.put(MessageRequestId(request.addr, receiver))
        except Exception:
            pass

        response = await receiver.get()

        if isinstance(response, RaftRespWrongLeader):
            leader_id, leader_addr = response.leader_id, response.leader_addr

            return raft_service_pb2.IdRequestResponse(
                code=raft_service_pb2.WrongLeader,
                data=pickle.dumps(tuple([leader_id, leader_addr, None])),
            )
        elif isinstance(response, RaftRespIdReserved):
            reserved_id = response.reserved_id
            peer_addrs = response.peer_addrs
            leader_id = response.leader_id

            return raft_service_pb2.IdRequestResponse(
                code=raft_service_pb2.Ok,
                data=pickle.dumps(tuple([leader_id, reserved_id, peer_addrs])),
            )
        else:
            assert False, "Unreachable"

    async def ChangeConfig(
        self, request: eraftpb_pb2.ConfChange, context: grpc.aio.ServicerContext
    ) -> raft_service_pb2.RaftResponse:
        receiver = Queue()
        await self.sender.put(MessageConfigChange(request, receiver))
        reply = raft_service_pb2.RaftResponse()

        try:
            if raft_response := await asyncio.wait_for(receiver.get(), 2):
                if isinstance(raft_response, RaftRespOk) or isinstance(
                    raft_response, RaftRespJoinSuccess
                ):
                    reply.inner = raft_response.encode()

        except asyncio.TimeoutError:
            reply.inner = RaftRespError().encode()
            logging.error("Timeout waiting for reply")

        finally:
            return reply

    async def SendMessage(
        self, request: eraftpb_pb2.Message, context: grpc.aio.ServicerContext
    ) -> raft_service_pb2.RaftResponse:
        await self.sender.put(MessageRaft(request))
        return raft_service_pb2.RaftResponse(inner=RaftRespOk().encode())

    async def RerouteMessage(
        self,
        request: raft_service_pb2.RerouteMessageArgs,
        context: grpc.aio.ServicerContext,
    ) -> raft_service_pb2.RaftResponse:
        receiver = Queue()

        await self.sender.put(
            MessageRerouteToLeader(
                confchange=request.conf_change,
                proposed_data=request.proposed_data,
                type=request.type,
                chan=receiver,
            )
        )
        reply = raft_service_pb2.RaftResponse()

        try:
            if raft_response := await asyncio.wait_for(receiver.get(), 2):
                if isinstance(raft_response, RaftRespResponse):
                    reply.inner = raft_response.data

        except asyncio.TimeoutError:
            reply.inner = RaftRespError().encode()
            logging.error("Timeout waiting for reply")

        finally:
            return reply
