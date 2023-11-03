import asyncio
import json
import pickle
from asyncio import Queue
from typing import Optional

import grpc

from .logger import AbstractRaftifyLogger
from .protos import eraftpb_pb2, raft_service_pb2, raft_service_pb2_grpc
from .raft_client import RaftClient
from .request_message import (
    ApplyConfigChangeForcelyReqMessage,
    ClusterBootstrapReadyReqMessage,
    ConfigChangeReqMessage,
    DebugEntriesReqMessage,
    DebugNodeReqMessage,
    MemberBootstrapReadyReqMessage,
    ProposeReqMessage,
    RaftReqMessage,
    RequestIdReqMessage,
    RerouteToLeaderReqMessage,
    VersionReqMessage,
)
from .response_message import (
    IdReservedRespMessage,
    JoinSuccessRespMessage,
    RaftErrorRespMessage,
    RaftRespMessage,
    SendMessageRespMessage,
    WrongLeaderRespMessage,
)
from .utils import SocketAddr


class RaftService(raft_service_pb2_grpc.RaftServiceServicer):
    """
    Network layer implementation using grpc.
    Public method names in this class use Pascal-case exceptionally.
    """

    def __init__(self, message_queue: Queue, logger: AbstractRaftifyLogger) -> None:
        self.message_queue = message_queue
        self.logger = logger
        self.confchange_req_queue: Queue[
            tuple[eraftpb_pb2.ConfChangeV2, bool]
        ] = Queue()
        self.confchange_req_applier = asyncio.create_task(
            self.__handle_confchange_request()
        )
        self.confchange_res_queue: Queue[
            raft_service_pb2.ChangeConfigResponse
        ] = Queue()

    async def __handle_reroute(
        self,
        response: WrongLeaderRespMessage,
        reroute_msg_type: raft_service_pb2.RerouteMsgType,
        proposed_data: Optional[bytes] = None,
        conf_change: Optional[eraftpb_pb2.ConfChangeV2] = None,
    ):
        leader_addr = response.leader_addr
        leader_client = RaftClient(SocketAddr.from_str(leader_addr))

        resp_from_leader = await leader_client.reroute_message(
            reroute_msg_type=reroute_msg_type,
            conf_change=conf_change,
            msg_bytes=proposed_data,
            timeout=2.0,
        )

        if isinstance(resp_from_leader, raft_service_pb2.SendMessageResponse):
            return resp_from_leader.data
        else:
            # TODO: handle this case. The leader might change in the meanwhile.
            assert False

    async def __handle_confchange_request(self):
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
                data=pickle.dumps({"leader_id": leader_id, "leader_addr": leader_addr}),
            )
        elif isinstance(response, IdReservedRespMessage):
            reserved_id = response.reserved_id
            peers = response.peers
            leader_id = response.leader_id

            return raft_service_pb2.IdRequestResponse(
                result=raft_service_pb2.IdRequest_Success,
                data=pickle.dumps(
                    {"leader_id": leader_id, "reserved_id": reserved_id, "peers": peers}
                ),
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
                if isinstance(raft_response, RaftRespMessage):
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

        match res.result:
            case raft_service_pb2.ChangeConfig_Success:
                return raft_service_pb2.ChangeConfigResponse(
                    data=res.data, result=res.result
                )
            case raft_service_pb2.ChangeConfig_WrongLeader:
                rerouted_resp = await self.__handle_reroute(
                    res,
                    reroute_msg_type=raft_service_pb2.ConfChange,
                    conf_change=request,
                )
                return raft_service_pb2.ChangeConfigResponse(
                    data=rerouted_resp, result=res.result
                )
            case raft_service_pb2.ChangeConfig_TimeoutError:
                return raft_service_pb2.ChangeConfigResponse(
                    data=res.data, result=res.result
                )
            case raft_service_pb2.ChangeConfig_UnknownError:
                return raft_service_pb2.ChangeConfigResponse(
                    data=b"", result=res.result
                )

        assert False

    async def ApplyConfigChangeForcely(
        self, request: eraftpb_pb2.ConfChangeV2, context: grpc.aio.ServicerContext
    ) -> raft_service_pb2.ChangeConfigResponse:
        await self.confchange_req_queue.put((request, True))
        return await self.confchange_res_queue.get()

    async def SendMessage(
        self, request: eraftpb_pb2.Message, context: grpc.aio.ServicerContext
    ) -> raft_service_pb2.SendMessageResponse:
        await self.message_queue.put(RaftReqMessage(request))
        return raft_service_pb2.SendMessageResponse(
            data=SendMessageRespMessage().encode()
        )

    async def Propose(
        self, request: raft_service_pb2.ProposeArgs, context: grpc.aio.ServicerContext
    ) -> raft_service_pb2.ProposeResponse:
        receiver: Queue = Queue()
        await self.message_queue.put(ProposeReqMessage(data=request.msg, chan=receiver))
        result = await receiver.get()

        if isinstance(result, WrongLeaderRespMessage):
            resp = await self.__handle_reroute(
                result,
                reroute_msg_type=raft_service_pb2.Propose,
                proposed_data=request.msg,
            )
            return raft_service_pb2.ProposeResponse(msg=resp)
        elif isinstance(result, RaftRespMessage):
            return raft_service_pb2.ProposeResponse(msg=result.data)

        assert False, f"Unknown type of response, resp: {resp}"

    async def ClusterBootstrapReady(
        self,
        request: raft_service_pb2.ClusterBootstrapReadyArgs,
        context: grpc.aio.ServicerContext,
    ) -> raft_service_pb2.ClusterBootstrapReadyResponse:
        receiver: Queue = Queue()
        await self.message_queue.put(
            ClusterBootstrapReadyReqMessage(peers=request.peers, chan=receiver)
        )
        _ = await receiver.get()

        return raft_service_pb2.ClusterBootstrapReadyResponse()

    async def MemberBootstrapReady(
        self,
        request: raft_service_pb2.MemberBootstrapReadyArgs,
        context: grpc.aio.ServicerContext,
    ) -> raft_service_pb2.MemberBootstrapReadyResponse:
        receiver: Queue = Queue()
        await self.message_queue.put(
            MemberBootstrapReadyReqMessage(
                follower_id=request.follower_id, chan=receiver
            )
        )
        _ = await receiver.get()

        return raft_service_pb2.MemberBootstrapReadyResponse()

    async def RerouteMessage(
        self,
        request: raft_service_pb2.RerouteMessageArgs,
        context: grpc.aio.ServicerContext,
    ) -> raft_service_pb2.SendMessageResponse:
        receiver: Queue = Queue()

        await self.message_queue.put(
            RerouteToLeaderReqMessage(
                conf_change=request.conf_change,
                proposed_data=request.proposed_data,
                type=request.type,
                chan=receiver,
            )
        )
        reply = raft_service_pb2.SendMessageResponse()

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

    async def DebugNode(
        self, _request: raft_service_pb2.Empty, _context: grpc.aio.ServicerContext
    ) -> raft_service_pb2.DebugNodeResponse:
        receiver: Queue = Queue()
        await self.message_queue.put(DebugNodeReqMessage(receiver))
        debug_info = await asyncio.wait_for(receiver.get(), 2)
        return raft_service_pb2.DebugNodeResponse(result=json.dumps(debug_info))

    async def DebugEntries(
        self, _request: raft_service_pb2.Empty, _context: grpc.aio.ServicerContext
    ) -> raft_service_pb2.DebugEntriesResponse:
        receiver: Queue = Queue()
        await self.message_queue.put(DebugEntriesReqMessage(receiver))
        all_entries = await asyncio.wait_for(receiver.get(), 2)
        return raft_service_pb2.DebugEntriesResponse(result=json.dumps(all_entries))

    async def Version(
        self, _request: raft_service_pb2.Empty, _context: grpc.aio.ServicerContext
    ) -> raft_service_pb2.VersionResponse:
        receiver: Queue = Queue()
        await self.message_queue.put(VersionReqMessage(receiver))
        result = await asyncio.wait_for(receiver.get(), 2)
        return raft_service_pb2.VersionResponse(result=result)
