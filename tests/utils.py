import json
import os
import signal

from tests.harness.constant import NODE_INFO_FILE_PATH


def read_json(path: str):
    with open(path, "r") as file:
        return json.load(file)


def write_json(path: str, dump: dict):
    with open(path, "w") as file:
        return json.dump(dump, file)


def read_pids(path: str) -> dict:
    data = read_json(path)
    nodes_data = data.get("nodes", [])
    pids_dict = {}

    for item in nodes_data:
        addr = item.get("addr")
        pid = item.get("pid")
        if addr and pid:
            pids_dict[addr] = pid

    return pids_dict


def write_pids(path: str, dump: dict):
    existing = read_json(path)
    nodes_list = [{"addr": k, "pid": v} for k, v in dump.items()]
    existing["nodes"] = nodes_list

    write_json(path, existing)


def kill_process(pid: int):
    try:
        os.kill(pid, signal.SIGKILL)
    except Exception:
        pass


def prepare():
    if os.path.exists(NODE_INFO_FILE_PATH):
        os.remove(NODE_INFO_FILE_PATH)

    write_json(NODE_INFO_FILE_PATH, {})


def killall():
    """
    Kill all the processes that spawned by the ProcessPoolExecutor and aiotools.start_server.
    """

    node_info = read_json(NODE_INFO_FILE_PATH)
    for node in node_info["nodes"]:
        kill_process(node["pid"])

    kill_process(node_info["root"]["pid"])
