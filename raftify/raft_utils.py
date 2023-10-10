import asyncio
from dataclasses import dataclass
from enum import Enum

from rraft import ConfChangeV2

from raftify.peers import Peers
from raftify.raft_client import RaftClient
from raftify.raft_node import RaftNode


async def leave_joint(raft_node: RaftNode):
    """
    Force Empty ConfChange entry to be committed.
    """
    # TODO: Execute commit on the more appropriate timing.
    # If possible, it would be great to switch to use "Auto" confchange transition.
    await asyncio.sleep(1)
    zero = ConfChangeV2.default()
    assert zero.leave_joint(), "Zero ConfChangeV2 must be empty"
    raft_node.raw_node.propose_conf_change_v2(b"", zero)


class RaftNodeRole(Enum):
    Leader = 0
    Follower = 1


@dataclass
class RequestIdResponse:
    follower_id: int
    leader: tuple[int, RaftClient]  # (leader_id, leader_client)
    peers: Peers
