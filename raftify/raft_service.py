import asyncio
import json
from asyncio import Queue
from typing import Optional

import grpc

from .codec.abc import AbstractCodec
from .config import RaftifyConfig
from .error import UnknownError
from .log_entry.abc import AbstractLogEntry
from .logger import AbstractRaftifyLogger
from .protos import eraftpb_pb2, raft_service_pb2, raft_service_pb2_grpc
from .raft_client import RaftClient
from .raft_client_response import ProposeResponse
from .request_message import (
    ApplyConfigChangeForcelyReqMessage,
    ClusterBootstrapReadyReqMessage,
    ConfigChangeReqMessage,
    DebugEntriesReqMessage,
    DebugNodeReqMessage,
    GetPeersReqMessage,
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
    PeerRemovalSuccessRespMessage,
    RaftErrorRespMessage,
    RaftRespMessage,
    WrongLeaderRespMessage,
)


class RaftService(raft_service_pb2_grpc.RaftServiceServicer):
    """
    Network layer implementation using grpc.
    Public method names in this class use Pascal-case exceptionally.
    """

    def __init__(
        self,
        message_queue: Queue,
        logger: AbstractRaftifyLogger,
        codec: AbstractCodec,
        cluster_config: RaftifyConfig,
    ) -> None:
        self.message_queue = message_queue
        self.logger = logger
        self.cluster_config = cluster_config
        self.codec = codec
        self.confchange_req_queue: Queue[
            tuple[eraftpb_pb2.ConfChangeV2, bool]
        ] = Queue()
        self.confchange_res_queue: Queue[
            raft_service_pb2.ChangeConfigResponse
        ] = Queue()

        self.confchange_req_applier = asyncio.create_task(
            self.__handle_confchange_request()
        )

    async def __handle_reroute(
        self,
        response: WrongLeaderRespMessage,
        reroute_msg_type: raft_service_pb2.RerouteMsgType,
        proposed_data: Optional[bytes] = None,
        conf_change: Optional[eraftpb_pb2.ConfChangeV2] = None,
    ) -> AbstractLogEntry:
        leader_client = RaftClient(response.leader_addr)

        rerouted_response = await leader_client.reroute_message(
            reroute_msg_type=reroute_msg_type,
            conf_change=conf_change,
            msg_bytes=proposed_data,
            timeout=5.0,
        )

        if isinstance(rerouted_response, ProposeResponse):
            return rerouted_response.msg
        else:
            # TODO: handle this case. The leader might change in the meanwhile.
            raise UnknownError("Not implemented login in rerouting")

    async def __handle_confchange_request(self):
        # TODO: Describe why this queueing is required.
        while True:
            request, force = await self.confchange_req_queue.get()
            response = await self.ChangeConfigRequestHandler(request, force)
            await self.confchange_res_queue.put(response)
            await asyncio.sleep(self.cluster_config.confchange_process_interval)

    async def RequestId(
        self,
        request: raft_service_pb2.IdRequestArgs,
        _context: grpc.aio.ServicerContext,
    ) -> raft_service_pb2.IdRequestResponse:
        receiver: Queue = Queue()
        await self.message_queue.put(RequestIdReqMessage(request.addr, receiver))
        response = await receiver.get()

        if isinstance(response, WrongLeaderRespMessage):
            return raft_service_pb2.IdRequestResponse(
                leader_id=response.leader_id,
                leader_addr=response.leader_addr,
                result=raft_service_pb2.IdRequest_WrongLeader,
            )
        elif isinstance(response, IdReservedRespMessage):
            return raft_service_pb2.IdRequestResponse(
                result=raft_service_pb2.IdRequest_Success,
                leader_id=response.leader_id,
                reserved_id=response.reserved_id,
                peers=response.raw_peers,
            )

        raise UnknownError(f"Unknown type of response, resp: {response}")

    async def ChangeConfigRequestHandler(
        self, request: eraftpb_pb2.ConfChangeV2, force: bool = False
    ) -> raft_service_pb2.ChangeConfigResponse:
        receiver: Queue = Queue()

        if force:
            await self.message_queue.put(
                ApplyConfigChangeForcelyReqMessage(request, receiver)
            )
        else:
            await self.message_queue.put(ConfigChangeReqMessage(request, receiver))

        reply = raft_service_pb2.ChangeConfigResponse()

        try:
            if response := await receiver.get():
                if isinstance(response, WrongLeaderRespMessage):
                    reply.result = raft_service_pb2.ChangeConfig_WrongLeader
                    leader_id, leader_addr = (
                        response.leader_id,
                        response.leader_addr,
                    )
                    reply.data = self.codec.encode(
                        {"leader_id": leader_id, "leader_addr": leader_addr}
                    )
                elif isinstance(response, JoinSuccessRespMessage) or isinstance(
                    response, PeerRemovalSuccessRespMessage
                ):
                    reply.result = raft_service_pb2.ChangeConfig_Success
                    reply.data = self.codec.encode(response)

        except asyncio.TimeoutError:
            reply.result = raft_service_pb2.ChangeConfig_TimeoutError
            reply.data = self.codec.encode(RaftErrorRespMessage(None))

            self.logger.error(
                'TimeoutError occurs while handling "ConfigChangeReqMessage!"'
            )

        except Exception as e:
            reply.result = raft_service_pb2.ChangeConfig_UnknownError
            reply.data = self.codec.encode(
                RaftErrorRespMessage(data=str(e).encode("utf-8"))
            )

        finally:
            return reply

    async def ChangeConfig(
        self, request: eraftpb_pb2.ConfChangeV2, _context: grpc.aio.ServicerContext
    ) -> raft_service_pb2.ChangeConfigResponse:
        await self.confchange_req_queue.put((request, False))

        response = await self.confchange_res_queue.get()

        if response.result == raft_service_pb2.ChangeConfig_WrongLeader:
            rerouted_response = await self.__handle_reroute(
                WrongLeaderRespMessage(**self.codec.decode(response.data)),
                reroute_msg_type=raft_service_pb2.ConfChange,
                conf_change=request,
            )
            return raft_service_pb2.ChangeConfigResponse(
                data=rerouted_response, result=response.result
            )

        return response

    async def ApplyConfigChangeForcely(
        self, request: eraftpb_pb2.ConfChangeV2, _context: grpc.aio.ServicerContext
    ) -> raft_service_pb2.ChangeConfigResponse:
        await self.confchange_req_queue.put((request, True))
        return await self.confchange_res_queue.get()

    async def SendMessage(
        self, request: eraftpb_pb2.Message, _context: grpc.aio.ServicerContext
    ) -> raft_service_pb2.Empty:
        await self.message_queue.put(RaftReqMessage(msg=request))
        return raft_service_pb2.Empty()

    async def Propose(
        self, request: raft_service_pb2.ProposeArgs, _context: grpc.aio.ServicerContext
    ) -> raft_service_pb2.ProposeResponse:
        receiver: Queue = Queue()
        await self.message_queue.put(
            ProposeReqMessage(data=request.msg, response_chan=receiver)
        )
        response = await receiver.get()

        if isinstance(response, WrongLeaderRespMessage):
            rerouted_response = await self.__handle_reroute(
                response,
                reroute_msg_type=raft_service_pb2.Propose,
                proposed_data=request.msg,
            )
            return raft_service_pb2.ProposeResponse(
                msg=rerouted_response, rejected=False
            )
        elif isinstance(response, RaftRespMessage):
            return raft_service_pb2.ProposeResponse(
                msg=response.data, rejected=response.rejected
            )

        raise UnknownError(f"Unknown type of response, resp: {response}")

    async def ClusterBootstrapReady(
        self,
        request: raft_service_pb2.ClusterBootstrapReadyArgs,
        _context: grpc.aio.ServicerContext,
    ) -> raft_service_pb2.ClusterBootstrapReadyResponse:
        receiver: Queue = Queue()
        await self.message_queue.put(
            ClusterBootstrapReadyReqMessage(peers=request.peers, response_chan=receiver)
        )
        _ = await receiver.get()

        return raft_service_pb2.ClusterBootstrapReadyResponse()

    async def MemberBootstrapReady(
        self,
        request: raft_service_pb2.MemberBootstrapReadyArgs,
        _context: grpc.aio.ServicerContext,
    ) -> raft_service_pb2.MemberBootstrapReadyResponse:
        receiver: Queue = Queue()
        await self.message_queue.put(
            MemberBootstrapReadyReqMessage(
                follower_id=request.follower_id, response_chan=receiver
            )
        )

        _ = await receiver.get()

        return raft_service_pb2.MemberBootstrapReadyResponse()

    async def RerouteMessage(
        self,
        request: raft_service_pb2.RerouteMessageArgs,
        _context: grpc.aio.ServicerContext,
    ) -> raft_service_pb2.ProposeResponse:
        receiver: Queue = Queue()

        await self.message_queue.put(
            RerouteToLeaderReqMessage(
                conf_change=request.conf_change,
                proposed_data=request.proposed_data,
                type=request.type,
                response_chan=receiver,
            )
        )
        reply = raft_service_pb2.ProposeResponse()

        try:
            response = await receiver.get()
            if isinstance(response, RaftRespMessage):
                reply.msg = response.data

        except asyncio.TimeoutError:
            reply.msg = self.codec.encode(RaftErrorRespMessage(None))
            self.logger.error('TimeoutError occurs while handling "RerouteMessage!"')

        finally:
            return reply

    async def DebugNode(
        self, _request: raft_service_pb2.Empty, _context: grpc.aio.ServicerContext
    ) -> raft_service_pb2.DebugNodeResponse:
        receiver: Queue = Queue()
        await self.message_queue.put(DebugNodeReqMessage(receiver))
        debug_info = await receiver.get()
        return raft_service_pb2.DebugNodeResponse(result=json.dumps(debug_info))

    async def DebugEntries(
        self, _request: raft_service_pb2.Empty, _context: grpc.aio.ServicerContext
    ) -> raft_service_pb2.DebugEntriesResponse:
        receiver: Queue = Queue()
        await self.message_queue.put(DebugEntriesReqMessage(receiver))
        all_entries = await receiver.get()
        return raft_service_pb2.DebugEntriesResponse(result=json.dumps(all_entries))

    async def Version(
        self, _request: raft_service_pb2.Empty, _context: grpc.aio.ServicerContext
    ) -> raft_service_pb2.VersionResponse:
        receiver: Queue = Queue()
        await self.message_queue.put(VersionReqMessage(receiver))
        result = await receiver.get()
        return raft_service_pb2.VersionResponse(result=result)

    async def GetPeers(
        self, _request: raft_service_pb2.Empty, _context: grpc.aio.ServicerContext
    ) -> raft_service_pb2.GetPeersResponse:
        receiver: Queue = Queue()
        await self.message_queue.put(GetPeersReqMessage(receiver))
        result = await receiver.get()
        return raft_service_pb2.GetPeersResponse(peers=result)
