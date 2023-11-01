import asyncio
import json
import sys

from .raft_client import RaftClient
from .raft_utils import format_all_entries, format_raft_node_debugging_info
from .utils import SocketAddr


def print_help():
    print(
        "Usage:\n"
        "  raftify-cli debug <ip:port> - Inspect a raft node at the specified IP address and port.\n"
        "  raftify-cli all-entries <ip:port> - Inspect entries of the raft node at the specified IP address and port.\n"
        "Examples:\n"
        "  raftify-cli debug 127.0.0.1:60061\n"
        "  raftify-cli all-entries 127.0.0.1:60061"
    )


async def async_main():
    if len(sys.argv) > 2:
        addr = SocketAddr.from_str(sys.argv[2])
        client = RaftClient(addr)

        match sys.argv[1]:
            case "all-entries":
                res = await client.debug_entries(timeout=5.0)
                print(format_all_entries(json.loads(res.result)))
                pass
            case "debug":
                res = await client.debug_node(timeout=5.0)
                print(format_raft_node_debugging_info(json.loads(res.result)))
            case "version":
                res = await client.version(timeout=5.0)
                print(res.result)
            case _:
                print_help()

    else:
        print_help()


def main():
    asyncio.run(async_main())
