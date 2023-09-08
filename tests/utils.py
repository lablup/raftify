import json
import os
import re
import shutil
import signal

import requests

from tests.harness.constant import CLUSTER_INFO_PATH, WEB_SERVER_ADDRS


def read_json(path: str) -> dict:
    with open(path, "r") as file:
        return json.load(file)


def write_json(path: str, data: dict):
    with open(path, "w") as file:
        json.dump(data, file)


def read_node(node_id: int) -> dict:
    return read_json(
        f"{CLUSTER_INFO_PATH}/.node-{node_id}.json",
    )


def write_node(node_id: int, data: dict):
    write_json(
        f"{CLUSTER_INFO_PATH}/.node-{node_id}.json",
        data,
    )


def remove_node(node_id: int):
    os.remove(f"{CLUSTER_INFO_PATH}/.node-{node_id}.json")


def read_cluster_info(path: str = CLUSTER_INFO_PATH) -> dict:
    result = {"root": {}, "nodes": []}

    node_pattern = re.compile(r"\.node-(\d+).json$")

    for filename in os.listdir(path):
        if filename == ".root.json":
            root_data = read_json(f"{path}/{filename}")
            result["root"] = root_data
        elif node_pattern.match(filename):
            node_data = read_json(f"{path}/{filename}")
            result["nodes"].append(node_data)

    return result


def write_cluster_info(cluster_info: dict, path: str = CLUSTER_INFO_PATH):
    for index, (addr, pid) in enumerate(cluster_info.items()):
        write_node(index + 1, {"addr": addr, "pid": pid})

    write_json(f"{path}/.root.json", {"root": cluster_info["root"]})


def get_cluster_size(path: str = CLUSTER_INFO_PATH) -> int:
    node_pattern = re.compile(r"\.node-(\d+).json$")
    count = sum(1 for filename in os.listdir(path) if node_pattern.match(filename))
    return count


def kill_process(pid: int):
    try:
        os.kill(pid, signal.SIGTERM)
    except Exception:
        pass


def killall():
    """
    Kill all the processes that spawned by the ProcessPoolExecutor and aiotools.start_server.
    """

    cluster = read_cluster_info()

    for node in cluster["nodes"]:
        kill_process(node["pid"])

    os.kill(cluster["root"]["pid"], signal.SIGKILL)


class RequestType:
    GET = "get"
    PUT = "put"


def make_request(typ: RequestType, node_id: int, request: str) -> str:
    res = requests.__dict__.get(typ)(f"http://{WEB_SERVER_ADDRS[node_id - 1]}{request}")
    return res.text


def reset_fixtures_directory():
    if os.path.exists(CLUSTER_INFO_PATH) and os.path.isdir(CLUSTER_INFO_PATH):
        shutil.rmtree(CLUSTER_INFO_PATH)

    os.makedirs(CLUSTER_INFO_PATH)
