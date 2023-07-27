import asyncio
from asyncio import Queue
from typing import Optional

from rraft import ConfChange, ConfChangeType

from riteraft.error import UnknownError
from riteraft.pb_adapter import ConfChangeAdapter
from riteraft.protos import raft_service_pb2
from riteraft.raft_node import RaftNode
from riteraft.request_message import MessageConfigChange, MessagePropose
from riteraft.response_message import (
    RaftRespOk,
    RaftResponse,
    RaftRespResponse,
    RaftRespWrongLeader,
)


class Mailbox:
    """
    A mailbox to send messages to a running raft node.
    """

    def __init__(self, raft_node: RaftNode, sender: Queue):
        self.sender = sender
        self.raft_node = raft_node

    async def __handle_response(
        self,
        response: RaftResponse,
        *,
        proposed_data: Optional[bytes] = None,
        confchange: Optional[raft_service_pb2.ConfChange] = None,
        reroute_msg_type: raft_service_pb2.RerouteMsgType,
    ) -> Optional[bytes]:
        if isinstance(response, RaftRespOk):
            return None
        if isinstance(response, RaftRespResponse):
            return response.data
        elif isinstance(response, RaftRespWrongLeader):
            leader_id = self.raft_node.get_leader_id()
            resp_from_leader = await self.raft_node.peers[leader_id].reroute_message(
                proposed_data, reroute_msg_type, confchange
            )
            if isinstance(resp_from_leader, raft_service_pb2.RaftResponse):
                return resp_from_leader.inner
            else:
                # TODO: handle this case. The leader might change in the meanwhile.
                assert False

        raise UnknownError(f"Unknown response type: {resp_from_leader}")

    async def send(self, message: bytes) -> bytes:
        """
        Send a proposal message to commit to the node.
        """

        receiver = Queue()
        # TODO: make timeout duration a variable
        await self.sender.put(MessagePropose(message, receiver))

        resp = await self.__handle_response(
            await asyncio.wait_for(receiver.get(), 2),
            reroute_msg_type=raft_service_pb2.Propose,
            proposed_data=message,
        )
        assert resp is not None
        return resp

    async def leave(self, node_id: int) -> None:
        cc = ConfChange.default()
        cc.set_node_id(node_id)
        cc.set_change_type(ConfChangeType.RemoveNode)

        receiver = Queue()
        confchange = ConfChangeAdapter.to_pb(cc)

        await self.sender.put(MessageConfigChange(confchange, receiver))

        await self.__handle_response(
            await receiver.get(),
            reroute_msg_type=raft_service_pb2.ConfChange,
            confchange=confchange,
        )
