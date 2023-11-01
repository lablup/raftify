import json
import os
import re
from dataclasses import dataclass
from enum import Enum
from typing import Any

from .peers import Peers
from .raft_client import RaftClient


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


def format_all_entries(all_entries: dict[str, Any]) -> str:
    """
    all_entries: result of raftify.raft_client.RaftClient.debug_entries
    """

    return f"""
========= Compacted all entries =========
{all_entries['compacted_all_entries']}

========= Existing all entries =========
{all_entries['current_all_entries']}
        """.strip()


def format_raft_node_debugging_info(debug_info: dict[str, Any]) -> str:
    """
    debug_info: result of raftify.raft_client.RaftClient.debug_node
    """

    return f"""
========= Node info =========
node_id: {debug_info['node_id']}
current_leader_id: {debug_info['current_leader_id']}

========= Persistence info =========
hard_state: {debug_info['storage']['hard_state']}
conf_state: {debug_info['storage']['conf_state']}
last_index: {debug_info['storage']['last_index']}
snapshot: {debug_info['storage']['snapshot']}

========= Progress tracker =========
progress: {debug_info['progress']}

========= Peer states =========
peer_states: {debug_info['peer_states']}

========= Raft log =========
last_applied: {debug_info['raft_log']['applied']}
last_committed: {debug_info['raft_log']['committed']}
last_persisted: {debug_info['raft_log']['persisted']}

========= Pending confchange =========
pending_conf_index: {debug_info['failure']['pending_conf_index']}
has_pending_conf: {debug_info['failure']['has_pending_conf']}
        """.strip()
