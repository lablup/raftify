import logging
from asyncio import Queue

from rraft import Message

from riteraft.raft_client import RaftClient
from riteraft.request_message import MessageReportUnreachable


class MessageSender:
    def __init__(
        self,
        message: Message,
        client: RaftClient,
        client_id: int,
        chan: Queue,
        max_retries: int,
        timeout: float = 5.0,
    ):
        self.message = message
        self.client = client
        self.client_id = client_id
        self.chan = chan
        self.max_retries = max_retries
        self.timeout = timeout

    async def send(self) -> None:
        """
        Attempt to send a message 'max_retries' times at 'timeout' interval.
        """

        current_retry = 0
        while True:
            try:
                await self.client.send_message(self.message, self.timeout)
                return
            except Exception:
                if current_retry < self.max_retries:
                    current_retry += 1
                else:
                    logging.debug(
                        f"Attempted to connect the {self.max_retries} retries, but were unable to establish a connection."
                    )

                    try:
                        await self.chan.put(MessageReportUnreachable(self.client_id))
                    except Exception:
                        pass
                    return
