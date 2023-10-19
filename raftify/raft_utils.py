import asyncio
import json
import os
import re
from dataclasses import dataclass
from enum import Enum
from typing import Any

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


def gather_compacted_logs(path: str) -> list[str]:
    result = []
    node_pattern = re.compile(r"compacted-logs-(\d+)\.json$")

    for filename in sorted(os.listdir(path)):
        match = node_pattern.match(filename)
        if match:
            with open(os.path.join(path, filename), "r") as file:
                result += json.load(file)

    return result


def get_all_entry_logs(raft_node: RaftNode) -> str:
    """
    Collect and return all entries in the raft log
    """
    current_all_entries = raft_node.raw_node.get_raft().get_raft_log().all_entries()
    current_all_entries_logs = "\n".join(
        list(map(lambda e: str(e), current_all_entries))
    )

    compacted_all_entries_logs = "\n".join(
        gather_compacted_logs(raft_node.lmdb.core.log_dir_path)
    )

    return f"""
========= Compacted all entries =========
{compacted_all_entries_logs}

========= Existing all entries =========
{current_all_entries_logs}
        """.strip()


# TODO: Move this into RaftNode if possible
def print_raft_node(debug_info: dict[str, Any]) -> str:
    return f"""
========= Node info =========
node_id: {debug_info['node_id']}
current_leader_id: {debug_info['current_leader_id']}

========= Storage info =========
hard_state: {debug_info['storage']['hard_state']}
conf_state: {debug_info['storage']['conf_state']}
last_index: {debug_info['storage']['last_index']}
snapshot: {debug_info['storage']['snapshot']}

========= Progress =========
progress: {debug_info['progress']}

========= Peer state =========
peer_states: {debug_info['peer_states']}

========= Raft log =========
applied: {debug_info['raft_log']['applied']}
committed: {debug_info['raft_log']['committed']}
persisted: {debug_info['raft_log']['persisted']}

========= Pending confchange =========
pending_conf_index: {debug_info['failure']['pending_conf_index']}
has_pending_conf: {debug_info['failure']['has_pending_conf']}
        """.strip()
