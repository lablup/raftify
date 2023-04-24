import logging
from asyncio import Queue, sleep

from rraft import Message

from riteraft.message import MessageReportUnreachable
from riteraft.raft_client import RaftClient


class MessageSender:
    def __init__(
        self,
        message: Message,
        client: RaftClient,
        client_id: int,
        chan: Queue,
        max_retries: int,
        timeout: float,
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
                await self.client.send_message(self.message)
                break
            except Exception as err:
                if current_retry < self.max_retries:
                    current_retry += 1
                    await sleep(self.timeout)
                else:
                    logging.debug(
                        f"Error sending message after {self.max_retries} retries: {err}"
                    )

                    await self.chan.send(
                        MessageReportUnreachable(node_id=self.client_id)
                    )
                    break
