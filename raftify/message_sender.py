from asyncio import Queue

from rraft import Message

from raftify.config import RaftifyConfig
from raftify.logger import AbstractRaftifyLogger
from raftify.raft_client import RaftClient
from raftify.request_message import ReportUnreachableReqMessage


class MessageSender:
    def __init__(
        self,
        message: Message,
        client: RaftClient,
        client_id: int,
        chan: Queue,
        logger: AbstractRaftifyLogger,
        raftify_cfg: RaftifyConfig,
    ):
        self.message = message
        self.client = client
        self.client_id = client_id
        self.chan = chan
        self.logger = logger
        self.raftify_cfg = raftify_cfg

    async def send(self) -> None:
        """
        Attempt to send a message 'max_retry_cnt' times at 'timeout' interval.
        """

        current_retry = 0
        while True:
            try:
                await self.client.send_message(
                    self.message, self.raftify_cfg.message_timeout
                )
                return
            except Exception:
                if current_retry < self.raftify_cfg.max_retry_cnt:
                    current_retry += 1
                else:
                    self.logger.debug(
                        f"Attempted to connect the {self.raftify_cfg.max_retry_cnt} retries, but were unable to establish a connection."
                    )

                    try:
                        await self.chan.put(ReportUnreachableReqMessage(self.client_id))
                    except Exception:
                        pass
                    return
