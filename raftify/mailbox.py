import pickle
from asyncio import Queue
from typing import Optional

from rraft import ConfChangeSingle, ConfChangeTransition, ConfChangeType, ConfChangeV2

from .error import ProposalRejectError, UnknownError
from .pb_adapter import ConfChangeV2Adapter
from .protos import eraftpb_pb2, raft_service_pb2
from .raft_node import RaftNode
from .request_message import ConfigChangeReqMessage, ProposeReqMessage
from .response_message import (
    ClusterBootstrapReadyRespMessage,
    ConfChangeSuccessRespMessage,
    RaftRespMessage,
    ResponseMessage,
    WrongLeaderRespMessage,
)


class Mailbox:
    """
    A mailbox to send messages to a running raft node.
    """

    def __init__(self, raft_node: RaftNode):
        self.raft_node = raft_node
        self.message_queue = raft_node.message_queue
        self.logger = raft_node.logger
        self.raftify_config = raft_node.raftify_cfg

    async def __handle_response(
        self,
        response: ResponseMessage,
        *,
        reroute_msg_type: Optional[raft_service_pb2.RerouteMsgType] = None,
        proposed_data: Optional[bytes] = None,
        conf_change: Optional[eraftpb_pb2.ConfChangeV2] = None,
    ) -> Optional[bytes]:
        if isinstance(response, ClusterBootstrapReadyRespMessage) or isinstance(
            response, ConfChangeSuccessRespMessage
        ):
            return None
        if isinstance(response, RaftRespMessage):
            if response.rejected:
                return None

            return response.data
        elif isinstance(response, WrongLeaderRespMessage):
            assert reroute_msg_type is not None

            leader_id = self.raft_node.get_leader_id()
            leader = self.raft_node.peers[leader_id]

            resp_from_leader = await leader.client.reroute_message(
                reroute_msg_type=reroute_msg_type,
                conf_change=conf_change,
                msg_bytes=proposed_data,
                timeout=self.raftify_config.message_timeout,
            )

            if isinstance(resp_from_leader, raft_service_pb2.SendMessageResponse):
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
            res = await self.__handle_response(
                await receiver.get(),
                reroute_msg_type=raft_service_pb2.Propose,
                proposed_data=message,
            )
        except Exception as e:
            self.logger.error("Error occurred while sending message through mailbox", e)
            raise

        if res is None:
            raise ProposalRejectError()

    async def leave(self, node_ids: int | list[int]) -> None:
        if isinstance(node_ids, int):
            node_ids = [node_ids]

        conf_change_v2 = ConfChangeV2.default()
        changes = []

        for node_id in node_ids:
            cc = ConfChangeSingle.default()
            cc.set_node_id(node_id)
            cc.set_change_type(ConfChangeType.RemoveNode)
            changes.append(cc)

        conf_change_v2.set_changes(changes)

        addrs = [self.raft_node.peers[node_id].addr for node_id in node_ids]
        conf_change_v2.set_context(pickle.dumps(addrs))
        conf_change_v2.set_transition(ConfChangeTransition.Auto)

        receiver: Queue = Queue()
        pb_conf_change_v2 = ConfChangeV2Adapter.to_pb(conf_change_v2)

        await self.message_queue.put(
            ConfigChangeReqMessage(pb_conf_change_v2, receiver)
        )

        try:
            await self.__handle_response(
                await receiver.get(),
                reroute_msg_type=raft_service_pb2.ConfChange,
                conf_change=pb_conf_change_v2,
            )
        except Exception as e:
            self.logger.error("Error occurred while sending message through mailbox", e)
            raise
