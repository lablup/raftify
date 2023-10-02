import asyncio

from rraft import ConfChangeV2

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
