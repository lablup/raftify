import json
import os
from dataclasses import dataclass
from enum import Enum
from typing import Any

from tabulate import tabulate

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


def append_to_json_file(dest_path: str, new_data: Any):
    os.makedirs(os.path.dirname(dest_path), exist_ok=True)

    try:
        with open(dest_path, "r", encoding="utf-8") as file:
            data = json.load(file)
            data.extend(new_data)
    except FileNotFoundError:
        data = new_data

    with open(dest_path, "w", encoding="utf-8") as file:
        json.dump(data, file, indent=4)


def print_all_entries(all_entries: dict[str, Any]) -> None:
    """
    all_entries: result of raftify.raft_client.RaftClient.debug_entries
    """

    print("========= Compacted entries =========")
    for entry in all_entries["compacted_all_entries"]:
        print(f"Key: {entry['index']}, Value: {entry}")

    print("========= Persisted entries =========")
    for entry in all_entries["persisted_entries"]:
        print(f"Key: {entry['index']}, Value: {entry}")


def format_raft_node_debugging_info(debug_info: dict[str, Any]) -> str:
    """
    debug_info: result of raftify.raft_client.RaftClient.debug_node
    """
    is_leader = debug_info["node_id"] == debug_info["current_leader_id"]

    peers_tbl = []
    for id, info in debug_info["peer_states"].items():
        row = [id, info["client"]["addr"], info["state"]]
        peers_tbl.append(row)

    return f"""
========= Outline =========
node_id: {debug_info['node_id']}
leader_id: {debug_info['current_leader_id']}

========= Persistence Info =========
{tabulate([
    ["Hard State", debug_info['storage']['hard_state']],
    ["Conf State", debug_info['storage']['conf_state']],
    ["Last Index", debug_info['storage']['last_index']],
    ["Snapshot", debug_info['storage']['snapshot']]
], headers=['Key', 'Value'], tablefmt="grid")}

========= Progresses =========
{tabulate(debug_info['progress'], headers="keys", tablefmt="grid") if is_leader else "(not leader node)"}

========= Peer states =========
{tabulate(peers_tbl, headers=['ID', 'Addr', 'State'], tablefmt="grid")}

========= RaftLog Metadata =========
last_applied: {debug_info['raft_log']['applied']}
last_committed: {debug_info['raft_log']['committed']}
last_persisted: {debug_info['raft_log']['persisted']}

========= Pending confchange =========
pending_conf_index: {debug_info['failure']['pending_conf_index']}
has_pending_conf: {debug_info['failure']['has_pending_conf']}
        """.strip()
