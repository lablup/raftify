import asyncio
import logging
from asyncio import Queue

import grpc
import msgpack
from rraft import ConfChange, Message

from riteraft.message import (
    MessageConfigChange,
    MessageRaft,
    MessageRequestId,
    RaftRespError,
    RaftRespIdReserved,
    RaftRespOk,
    RaftResponse,
    RaftRespWrongLeader,
)
from riteraft.protos import eraftpb_pb2, raft_service_pb2


class RaftService:
    def __init__(self, sender: Queue) -> None:
        self.sender = sender

    async def RequestId(
        self, request: raft_service_pb2.Empty, context: grpc.aio.ServicerContext
    ) -> raft_service_pb2.IdRequestResponse:
        chan = Queue()
        await self.sender.put(MessageRequestId(chan))
        response = await chan.get()

        if isinstance(response, RaftRespWrongLeader):
            logging.warning("Sending wrong leader")
            leader_id, leader_addr = response.leader_id, response.leader_addr

            return raft_service_pb2.IdRequestResponse(
                code=raft_service_pb2.WrongLeader,
                data=msgpack.packb(tuple([leader_id, leader_addr])),
            )
        elif isinstance(response, RaftRespIdReserved):
            id = response.id

            return raft_service_pb2.IdRequestResponse(
                code=raft_service_pb2.Ok, data=msgpack.packb(tuple([1, id]))
            )
        else:
            assert False, "Unreachable"

    async def ChangeConfig(
        self, request: eraftpb_pb2.ConfChange, context: grpc.aio.ServicerContext
    ) -> raft_service_pb2.RaftResponse:
        # TODO: Avoid below manual object creations if possible
        change = ConfChange.default()

        change.set_id(request.id)
        change.set_node_id(request.node_id)
        change.set_change_type(request.change_type)
        change.set_context(request.context)

        chan = Queue()
        await self.sender.put(MessageConfigChange(change, chan))
        reply = raft_service_pb2.RaftResponse(None)

        try:
            if raft_response := await asyncio.wait_for(chan.get(), 2):
                if isinstance(raft_response, RaftResponse):
                    reply.inner = raft_response.dumps()

        except asyncio.TimeoutError:
            reply.inner = msgpack.packb(RaftRespError())
            logging.error("Timeout waiting for reply")

        finally:
            return reply

    async def SendMessage(
        self, request: eraftpb_pb2.Message, context: grpc.aio.ServicerContext
    ) -> raft_service_pb2.RaftResponse:
        message = Message.default()

        message.set_commit(request.commit)
        message.set_context(request.context)
        message.set_entries(request.entries)
        message.set_index(request.index)
        message.set_log_term(request.log_term)
        message.set_msg_type(request.msg_type)
        message.set_priority(request.priority)
        message.set_reject(request.reject)
        message.set_reject_hint(request.reject_hint)
        message.set_request_snapshot(request.request_snapshot)
        message.set_snapshot(request.snapshot)
        message.set_term(request.term)
        message.set_to(request.to)
        # message.set_commit_term()
        # message.set_from()

        await self.sender.put(MessageRaft(message))
        return raft_service_pb2.RaftResponse(msgpack.packb(RaftRespOk()))
