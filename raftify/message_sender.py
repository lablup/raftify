import pickle
from asyncio import Queue

from rraft import ConfChange, ConfChangeType, Message, RawNode

from raftify.config import RaftifyConfig
from raftify.logger import AbstractRaftifyLogger
from raftify.raft_client import RaftClient
from raftify.request_message import ReportUnreachableReqMessage
from raftify.utils import AtomicInteger


class MessageSender:
    def __init__(
        self,
        message: Message,
        client: RaftClient,
        chan: Queue,
        logger: AbstractRaftifyLogger,
        raftify_cfg: RaftifyConfig,
        seq: AtomicInteger,
        peers: dict[int, RaftClient],
        raw_node: RawNode,
    ):
        self.message = message
        self.client = client
        self.chan = chan
        self.logger = logger
        self.raftify_cfg = raftify_cfg
        self.peers = peers
        self.raw_node = raw_node
        self.seq = seq

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

                    client_id = self.message.get_to()

                    try:
                        failed_request_counter = self.peers[
                            client_id
                        ].failed_request_counter

                        if failed_request_counter.value >= 3:
                            self.logger.debug(
                                f"Removed 'Node {client_id}' from cluster automatically because the request kept failed"
                            )

                            del self.peers[client_id]

                            conf_change = ConfChange.default()
                            conf_change.set_node_id(client_id)
                            conf_change.set_context(pickle.dumps(self.client.addr))
                            conf_change.set_change_type(ConfChangeType.RemoveNode)
                            self.raw_node.propose_conf_change(pickle.dumps(self.seq.value), conf_change)
                        else:
                            await self.chan.put(ReportUnreachableReqMessage(client_id))
                            failed_request_counter.increase()
                    except Exception:
                        pass
                    return
