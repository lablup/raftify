import asyncio
from asyncio import Queue

from rraft import ConfChange, ConfChangeType

from riteraft.error import UnknownError
from riteraft.message import (
    MessageConfigChange,
    MessagePropose,
    RaftRespOk,
    RaftRespResponse,
)


class Mailbox:
    """
    A mailbox to send messages to a running raft node.
    """

    def __init__(self, sender: Queue):
        self.__sender = sender

    async def send(self, message: bytes) -> bytes:
        """
        Send a proposal message to commit to the node.
        This fails if the current node is not the leader.
        """

        receiver = Queue()
        # TODO make timeout duration a variable
        await self.__sender.put(MessagePropose(message, receiver))

        resp = None
        try:
            resp = await asyncio.wait_for(receiver.get(), 2)
        except Exception as e:
            raise UnknownError(str(e))

        if isinstance(resp, RaftRespResponse):
            return resp.data
        else:
            raise UnknownError(f"Unknown response data: {resp}")

    async def leave(self) -> None:
        change = ConfChange.default()
        # set node id to 0, the node will set it to self when it receives it.
        change.set_node_id(0)
        change.set_change_type(ConfChangeType.RemoveNode)

        receiver = Queue()
        if await self.__sender.put(MessageConfigChange(change, receiver)):
            resp = await receiver.get()
            if isinstance(resp, RaftRespOk):
                return
            else:
                raise UnknownError(f"Unknown response data: {resp}")
