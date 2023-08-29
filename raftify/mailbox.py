import asyncio
import pickle
from asyncio import Queue
from typing import Optional

from rraft import ConfChange, ConfChangeType

from raftify.error import UnknownError
from raftify.pb_adapter import ConfChangeAdapter
from raftify.protos import raft_service_pb2
from raftify.raft_node import RaftNode
from raftify.request_message import ConfigChangeReqMessage, ProposeReqMessage
from raftify.response_message import (
    RaftOkRespMessage,
    RaftRespMessage,
    RaftResponse,
    WrongLeaderRespMessage,
)
from raftify.utils import SocketAddr


class Mailbox:
    """
    A mailbox to send messages to a running raft node.
    """

    def __init__(self, addr: SocketAddr, raft_node: RaftNode, sender: Queue):
        self.sender = sender
        self.raft_node = raft_node
        self.addr = addr

    async def __handle_response(
        self,
        response: RaftResponse,
        *,
        reroute_msg_type: raft_service_pb2.RerouteMsgType,
        proposed_data: Optional[bytes] = None,
        confchange: Optional[raft_service_pb2.ConfChange] = None,
    ) -> Optional[bytes]:
        if isinstance(response, RaftOkRespMessage):
            return None
        if isinstance(response, RaftRespMessage):
            return response.data
        elif isinstance(response, WrongLeaderRespMessage):
            leader_id = self.raft_node.get_leader_id()
            resp_from_leader = await self.raft_node.peers[leader_id].reroute_message(
                reroute_msg_type=reroute_msg_type,
                confchange=confchange,
                msg_bytes=proposed_data,
            )
            if isinstance(resp_from_leader, raft_service_pb2.RaftMessageResponse):
                return resp_from_leader.data
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
        await self.sender.put(ProposeReqMessage(message, receiver))

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
        cc.set_context(pickle.dumps(self.addr))
        cc.set_change_type(ConfChangeType.RemoveNode)

        receiver = Queue()
        confchange = ConfChangeAdapter.to_pb(cc)

        await self.sender.put(ConfigChangeReqMessage(confchange, receiver))

        await self.__handle_response(
            await receiver.get(),
            reroute_msg_type=raft_service_pb2.ConfChange,
            confchange=confchange,
        )
