import asyncio
from asyncio import Queue

from rraft import ConfChange, ConfChangeType

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

    async def send(self, message: bytes) -> RaftRespResponse:
        """
        Send a proposal message to commit to the node.
        This fails if the current node is not the leader.
        """

        receiver = Queue()
        # TODO make timeout duration a variable
        await self.__sender.put(MessagePropose(message, receiver))
        print("below func is failing")
        data = await asyncio.wait_for(receiver.get(), 2)
        if isinstance(data, RaftRespResponse):
            return data
        else:
            raise Exception("Unknown error")

    async def leave(self) -> None:
        change = ConfChange.default()
        # set node id to 0, the node will set it to self when it receives it.
        change.set_node_id(0)
        change.set_change_type(ConfChangeType.RemoveNode)

        receiver = Queue()
        if await self.__sender.put(MessageConfigChange(change, receiver)):
            data = await receiver.get()
            if isinstance(data, RaftRespOk):
                return
            else:
                raise Exception("Unknown error")
