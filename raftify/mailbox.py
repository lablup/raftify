import asyncio
import pickle
from asyncio import Queue
from typing import Optional

from rraft import ConfChangeSingle, ConfChangeTransition, ConfChangeType, ConfChangeV2

from raftify.config import RaftifyConfig
from raftify.error import UnknownError
from raftify.logger import AbstractRaftifyLogger
from raftify.pb_adapter import ConfChangeV2Adapter
from raftify.protos import eraftpb_pb2, raft_service_pb2
from raftify.raft_node import RaftNode
from raftify.raft_utils import leave_joint
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

    def __init__(
        self,
        addr: SocketAddr,
        raft_node: RaftNode,
        message_queue: Queue,
        logger: AbstractRaftifyLogger,
        raftify_config: RaftifyConfig,
    ):
        self.message_queue = message_queue
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
        conf_change: Optional[eraftpb_pb2.ConfChangeV2] = None,
    ) -> Optional[bytes]:
        if isinstance(response, RaftOkRespMessage):
            return None
        if isinstance(response, RaftRespMessage):
            return response.data
        elif isinstance(response, WrongLeaderRespMessage):
            assert reroute_msg_type is not None

            leader_id = self.raft_node.get_leader_id()
            leader = self.raft_node.peers[leader_id]

            assert leader and leader.client is not None

            resp_from_leader = await leader.client.reroute_message(
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

        raise UnknownError(f"Unknown response type: {response}")

    async def send(self, message: bytes) -> bytes:
        """
        Send a proposal message to commit to the node.
        """

        receiver: Queue = Queue()
        # TODO: make timeout duration a variable
        await self.message_queue.put(ProposeReqMessage(message, receiver))

        try:
            resp = await self.__handle_response(
                await asyncio.wait_for(receiver.get(), 2),
                reroute_msg_type=raft_service_pb2.Propose,
                proposed_data=message,
            )
            assert resp is not None
            return resp
        except Exception as e:
            self.logger.error("Error occurred while sending message through mailbox", e)
            raise

    async def leave(self, node_ids: list[int], addrs: list[SocketAddr]) -> None:
        assert len(node_ids) == len(addrs)

        conf_change_v2 = ConfChangeV2.default()
        changes = []
        for node_id in node_ids:
            cc = ConfChangeSingle.default()
            cc.set_node_id(node_id)
            cc.set_change_type(ConfChangeType.RemoveNode)
            changes.append(cc)

        conf_change_v2.set_changes(changes)
        conf_change_v2.set_context(pickle.dumps(addrs))
        conf_change_v2.set_transition(ConfChangeTransition.Auto)

        receiver: Queue = Queue()
        pb_conf_change_v2 = ConfChangeV2Adapter.to_pb(conf_change_v2)

        await self.message_queue.put(
            ConfigChangeReqMessage(pb_conf_change_v2, receiver)
        )

        res = await receiver.get()

        try:
            resp = await self.__handle_response(
                res,
                reroute_msg_type=raft_service_pb2.ConfChange,
                conf_change=pb_conf_change_v2,
            )
            assert resp is None
        except Exception as e:
            self.logger.error("Error occurred while sending message through mailbox", e)
            raise

        # for node_id in node_ids:
        #     self.raft_node.peers.data.pop(node_id, None)

        # asyncio.create_task(leave_joint(self.raft_node))
