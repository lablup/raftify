import asyncio
import pickle
from asyncio import Queue

import grpc

from raftify.logger import AbstractRaftifyLogger
from raftify.protos import eraftpb_pb2, raft_service_pb2, raft_service_pb2_grpc
from raftify.request_message import (
    ApplyConfigChangeForcelyReqMessage,
    ClusterBootstrapReadyReqMessage,
    ConfigChangeReqMessage,
    MemberBootstrapReadyReqMessage,
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
    def __init__(self, message_queue: Queue, logger: AbstractRaftifyLogger) -> None:
        self.message_queue = message_queue
        self.logger = logger
        self.confchange_req_queue: Queue = Queue()
        self.confchange_req_applier = asyncio.create_task(
            self.confchange_request_applier()
        )
        self.confchange_res_queue: Queue = Queue()

    async def confchange_request_applier(self):
        # TODO: Describe why this queueing is required.
        while True:
            req, force = await self.confchange_req_queue.get()
            res = await self.ChangeConfigRequestHandler(req, force)
            await self.confchange_res_queue.put(res)
            await asyncio.sleep(1)

    async def RequestId(
        self, request: raft_service_pb2.IdRequestArgs, context: grpc.aio.ServicerContext
    ) -> raft_service_pb2.IdRequestResponse:
        receiver: Queue = Queue()
        await self.message_queue.put(RequestIdReqMessage(request.addr, receiver))
        response = await receiver.get()

        if isinstance(response, WrongLeaderRespMessage):
            leader_id, leader_addr = response.leader_id, response.leader_addr

            return raft_service_pb2.IdRequestResponse(
                result=raft_service_pb2.IdRequest_WrongLeader,
                data=pickle.dumps(tuple([leader_id, leader_addr, None])),
            )
        elif isinstance(response, IdReservedRespMessage):
            reserved_id = response.reserved_id
            peers = response.peers
            leader_id = response.leader_id

            return raft_service_pb2.IdRequestResponse(
                result=raft_service_pb2.IdRequest_Success,
                data=pickle.dumps(tuple([leader_id, reserved_id, peers])),
            )
        else:
            assert False, "Unreachable"

    async def ChangeConfigRequestHandler(
        self, request: eraftpb_pb2.ConfChangeV2, force: bool = False
    ):
        receiver: Queue = Queue()

        if force:
            await self.message_queue.put(
                ApplyConfigChangeForcelyReqMessage(request, receiver)
            )
        else:
            await self.message_queue.put(ConfigChangeReqMessage(request, receiver))

        reply = raft_service_pb2.ChangeConfigResponse()

        try:
            # Does putting timeout here make sense?
            # ChangeConfig request should send response anyway.
            # Current implementation send response only after the conf change is properly committed.
            # TODO: timeout should be configurable
            if raft_response := await asyncio.wait_for(receiver.get(), 2):
                if isinstance(raft_response, RaftOkRespMessage):
                    reply.result = raft_service_pb2.ChangeConfig_Success
                elif isinstance(raft_response, JoinSuccessRespMessage):
                    reply.result = raft_service_pb2.ChangeConfig_Success
                    reply.data = raft_response.encode()
                elif isinstance(raft_response, WrongLeaderRespMessage):
                    reply.result = raft_service_pb2.ChangeConfig_WrongLeader
                    leader_id, leader_addr = (
                        raft_response.leader_id,
                        raft_response.leader_addr,
                    )
                    reply.data = pickle.dumps(tuple([leader_id, leader_addr, None]))

        except asyncio.TimeoutError:
            reply.result = raft_service_pb2.ChangeConfig_TimeoutError
            reply.data = RaftErrorRespMessage().encode()

            self.logger.error(
                'TimeoutError occurs while handling "ConfigChangeReqMessage!"'
            )

        except Exception as e:
            reply.result = raft_service_pb2.ChangeConfig_UnknownError
            reply.data = RaftErrorRespMessage(data=str(e).encode("utf-8")).encode()

        finally:
            return reply

    async def ChangeConfig(
        self, request: eraftpb_pb2.ConfChangeV2, context: grpc.aio.ServicerContext
    ) -> raft_service_pb2.ChangeConfigResponse:
        await self.confchange_req_queue.put((request, False))
        res = await self.confchange_res_queue.get()
        return res

    async def ApplyConfigChangeForcely(
        self, request: eraftpb_pb2.ConfChangeV2, context: grpc.aio.ServicerContext
    ) -> raft_service_pb2.ChangeConfigResponse:
        await self.confchange_req_queue.put((request, True))
        res = await self.confchange_res_queue.get()
        return res

    async def SendMessage(
        self, request: eraftpb_pb2.Message, context: grpc.aio.ServicerContext
    ) -> raft_service_pb2.RaftMessageResponse:
        await self.message_queue.put(RaftReqMessage(request))
        return raft_service_pb2.RaftMessageResponse(data=RaftOkRespMessage().encode())

    async def ClusterBootstrapReady(
        self,
        request: raft_service_pb2.ClusterBootstrapReadyArgs,
        context: grpc.aio.ServicerContext,
    ) -> raft_service_pb2.RaftMessageResponse:
        receiver: Queue = Queue()
        await self.message_queue.put(
            ClusterBootstrapReadyReqMessage(peers=request.peers, chan=receiver)
        )
        _ = await receiver.get()

        return raft_service_pb2.RaftMessageResponse(data=RaftOkRespMessage().encode())

    async def MemberBootstrapReady(
        self,
        request: raft_service_pb2.MemberBootstrapReadyArgs,
        context: grpc.aio.ServicerContext,
    ) -> raft_service_pb2.RaftMessageResponse:
        receiver: Queue = Queue()
        await self.message_queue.put(
            MemberBootstrapReadyReqMessage(
                follower_id=request.follower_id, chan=receiver
            )
        )
        _ = await receiver.get()

        return raft_service_pb2.RaftMessageResponse(data=RaftOkRespMessage().encode())

    async def RerouteMessage(
        self,
        request: raft_service_pb2.RerouteMessageArgs,
        context: grpc.aio.ServicerContext,
    ) -> raft_service_pb2.RaftMessageResponse:
        receiver: Queue = Queue()

        await self.message_queue.put(
            RerouteToLeaderReqMessage(
                conf_change=request.conf_change,
                proposed_data=request.proposed_data,
                type=request.type,
                chan=receiver,
            )
        )
        reply = raft_service_pb2.RaftMessageResponse()

        try:
            # TODO: timeout should be configurable
            if raft_response := await asyncio.wait_for(receiver.get(), 2):
                if isinstance(raft_response, RaftRespMessage):
                    reply.data = raft_response.data

        except asyncio.TimeoutError:
            reply.data = RaftErrorRespMessage().encode()
            self.logger.error('TimeoutError occurs while handling "RerouteMessage!"')

        finally:
            return reply
