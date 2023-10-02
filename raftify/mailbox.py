import asyncio
import pickle
from asyncio import Queue
from typing import Optional

from rraft import ConfChange, ConfChangeType

from raftify.config import RaftifyConfig
from raftify.error import UnknownError
from raftify.logger import AbstractRaftifyLogger
from raftify.pb_adapter import ConfChangeV2Adapter
from raftify.protos import raft_service_pb2
from raftify.raft_node import RaftNode
from raftify.request_message import (
    ClusteBootstrapReadyReqMessage,
    ConfigChangeReqMessage,
    MemberBootstrapReadyReqMessage,
    ProposeReqMessage,
)
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

    def __init__(
        self,
        addr: SocketAddr,
        raft_node: RaftNode,
        sender: Queue,
        logger: AbstractRaftifyLogger,
        raftify_config: RaftifyConfig,
    ):
        self.sender = sender
        self.raft_node = raft_node
        self.addr = addr
        self.logger = logger
        self.raftify_config = raftify_config

    async def __handle_response(
        self,
        response: RaftResponse,
        *,
        reroute_msg_type: Optional[raft_service_pb2.RerouteMsgType] = None,
        proposed_data: Optional[bytes] = None,
        conf_change: Optional[raft_service_pb2.ConfChange] = None,
    ) -> Optional[bytes]:
        if isinstance(response, RaftOkRespMessage):
            return None
        if isinstance(response, RaftRespMessage):
            return response.data
        elif isinstance(response, WrongLeaderRespMessage):
            assert reroute_msg_type is not None

            leader_id = self.raft_node.get_leader_id()
            resp_from_leader = await self.raft_node.peers[leader_id].reroute_message(
                reroute_msg_type=reroute_msg_type,
                conf_change=conf_change,
                msg_bytes=proposed_data,
                timeout=self.raftify_config.message_timeout,
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

        receiver: Queue = Queue()
        # TODO: make timeout duration a variable
        await self.sender.put(ProposeReqMessage(message, receiver))

        try:
            resp = await self.__handle_response(
                await asyncio.wait_for(receiver.get(), 2),
                reroute_msg_type=raft_service_pb2.Propose,
                proposed_data=message,
            )
            assert resp is not None
            return resp
        except Exception as e:
            self.logger.error("Error occured while sending message through mailbox", e)

    async def leave(self, node_id: int) -> None:
        conf_change = ConfChange.default()
        conf_change.set_node_id(node_id)
        conf_change.set_context(pickle.dumps(self.addr))
        conf_change.set_change_type(ConfChangeType.RemoveNode)
        conf_change_v2 = conf_change.as_v2()

        receiver: Queue = Queue()
        conf_change_v2 = ConfChangeV2Adapter.to_pb(conf_change_v2)

        await self.sender.put(ConfigChangeReqMessage(conf_change_v2, receiver))

        await self.__handle_response(
            await receiver.get(),
            reroute_msg_type=raft_service_pb2.ConfChange,
            conf_change=conf_change_v2,
        )

    async def bootstrap_ready(self) -> None:
        receiver: Queue = Queue()
        raw_peers = self.raft_node.peers.encode()
        await self.sender.put(ClusteBootstrapReadyReqMessage(raw_peers, receiver))
        await self.__handle_response(
            await receiver.get(),
        )

    async def member_bootstrap_ready(self, follower_id: int) -> None:
        receiver: Queue = Queue()
        await self.sender.put(MemberBootstrapReadyReqMessage(follower_id, receiver))
        await self.__handle_response(
            await receiver.get(),
        )
