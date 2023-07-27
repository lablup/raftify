import asyncio
from asyncio import Queue

from rraft import ConfChange, ConfChangeType

from riteraft.error import UnknownError
from riteraft.pb_adapter import ConfChangeAdapter
from riteraft.protos import raft_service_pb2
from riteraft.raft_node import RaftNode
from riteraft.request_message import MessageConfigChange, MessagePropose
from riteraft.response_message import RaftRespOk, RaftRespResponse, RaftRespWrongLeader


class Mailbox:
    """
    A mailbox to send messages to a running raft node.
    """

    def __init__(self, raft_node: RaftNode, sender: Queue):
        self.sender = sender
        self.raft_node = raft_node

    async def send(self, message: bytes) -> bytes:
        """
        Send a proposal message to commit to the node.
        # TODO: This should not fail. Instead it should reroute the message to the leader.
        This fails if the current node is not the leader.
        """

        leader_id = self.raft_node.get_leader_id()

        receiver = Queue()
        # TODO: make timeout duration a variable
        await self.sender.put(MessagePropose(message, receiver))

        resp = None
        try:
            resp = await asyncio.wait_for(receiver.get(), 2)
        except Exception as e:
            raise UnknownError(str(e))

        if isinstance(resp, RaftRespResponse):
            return resp.data
        elif isinstance(resp, RaftRespWrongLeader):
            resp_from_leader = await self.raft_node.peers[leader_id].reroute_message(
                message
            )
            if isinstance(resp_from_leader, raft_service_pb2.RaftResponse):
                return resp_from_leader.inner
            else:
                # TODO: handle this case
                assert False, "Unreachable"
        else:
            raise UnknownError(f"Unknown response data: {resp}")

    async def leave(self) -> None:
        change = ConfChange.default()
        # set node id to 0, the node will set it to self when it receives it.
        change.set_node_id(0)
        change.set_change_type(ConfChangeType.RemoveNode)

        receiver = Queue()
        await self.sender.put(
            MessageConfigChange(ConfChangeAdapter.to_pb(change), receiver)
        )

        resp = await receiver.get()

        if isinstance(resp, RaftRespOk):
            return
        else:
            raise UnknownError(f"Unknown response data: {resp}")
