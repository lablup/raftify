import asyncio
import pickle
from asyncio import Queue

import grpc

from raftify.logger import AbstractRaftifyLogger
from raftify.protos import eraftpb_pb2, raft_service_pb2, raft_service_pb2_grpc
from raftify.request_message import (
    ConfigChangeReqMessage,
    RaftReqMessage,
    RequestIdReqMessage,
    RerouteToLeaderReqMessage,
)
from raftify.response_message import (
    IdReservedRespMessage,
    JoinSuccessRespMessage,
    RaftErrorRespMessage,
    RaftOkRespMessage,
    RaftRespMessage,
    WrongLeaderRespMessage,
)


class RaftService(raft_service_pb2_grpc.RaftServiceServicer):
    def __init__(self, sender: Queue, logger: AbstractRaftifyLogger) -> None:
        self.sender = sender
        self.logger = logger

    async def RequestId(
        self, request: raft_service_pb2.IdRequestArgs, context: grpc.aio.ServicerContext
    ) -> raft_service_pb2.IdRequestResponse:
        receiver = Queue()
        try:
            await self.sender.put(RequestIdReqMessage(request.addr, receiver))
        except Exception:
            pass

        response = await receiver.get()

        if isinstance(response, WrongLeaderRespMessage):
            leader_id, leader_addr = response.leader_id, response.leader_addr

            return raft_service_pb2.IdRequestResponse(
                result=raft_service_pb2.IdRequest_WrongLeader,
                data=pickle.dumps(tuple([leader_id, leader_addr, None])),
            )
        elif isinstance(response, IdReservedRespMessage):
            reserved_id = response.reserved_id
            peer_addrs = response.peer_addrs
            leader_id = response.leader_id

            return raft_service_pb2.IdRequestResponse(
                result=raft_service_pb2.IdRequest_Success,
                data=pickle.dumps(tuple([leader_id, reserved_id, peer_addrs])),
            )
        else:
            assert False, "Unreachable"

    async def ChangeConfig(
        self, request: eraftpb_pb2.ConfChange, context: grpc.aio.ServicerContext
    ) -> raft_service_pb2.ChangeConfigResponse:
        receiver = Queue()
        await self.sender.put(ConfigChangeReqMessage(request, receiver))
        reply = raft_service_pb2.ChangeConfigResponse()

        try:
            if raft_response := await asyncio.wait_for(receiver.get(), 2):
                if isinstance(raft_response, RaftOkRespMessage):
                    reply.data = b""
                if isinstance(raft_response, JoinSuccessRespMessage):
                    reply.data = raft_response.encode()
                elif isinstance(raft_response, WrongLeaderRespMessage):
                    leader_id, leader_addr = (
                        raft_response.leader_id,
                        raft_response.leader_addr,
                    )
                    reply.data = (pickle.dumps(tuple([leader_id, leader_addr, None])),)

        except asyncio.TimeoutError:
            reply.result = raft_service_pb2.ChangeConfig_TimeoutError
            reply.data = RaftErrorRespMessage().encode()
            self.logger.error("Timeout waiting for reply")

        except grpc.aio.AioRpcError as e:
            reply.result = raft_service_pb2.ChangeConfig_GrpcError
            reply.data = RaftErrorRespMessage(data=str(e).encode("utf-8")).encode()

        except Exception as e:
            reply.result = raft_service_pb2.ChangeConfig_UnknownError
            reply.data = RaftErrorRespMessage(data=str(e).encode("utf-8")).encode()

        finally:
            return reply

    async def SendMessage(
        self, request: eraftpb_pb2.Message, context: grpc.aio.ServicerContext
    ) -> raft_service_pb2.RaftMessageResponse:
        await self.sender.put(RaftReqMessage(request))
        return raft_service_pb2.RaftMessageResponse(data=RaftOkRespMessage().encode())

    async def RerouteMessage(
        self,
        request: raft_service_pb2.RerouteMessageArgs,
        context: grpc.aio.ServicerContext,
    ) -> raft_service_pb2.RaftMessageResponse:
        receiver = Queue()

        await self.sender.put(
            RerouteToLeaderReqMessage(
                confchange=request.conf_change,
                proposed_data=request.proposed_data,
                type=request.type,
                chan=receiver,
            )
        )
        reply = raft_service_pb2.RaftMessageResponse()

        try:
            if raft_response := await asyncio.wait_for(receiver.get(), 2):
                if isinstance(raft_response, RaftRespMessage):
                    reply.data = raft_response.data

        except asyncio.TimeoutError:
            reply.data = RaftErrorRespMessage().encode()
            self.logger.error("Timeout waiting for reply")

        finally:
            return reply
