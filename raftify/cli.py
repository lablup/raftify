import asyncio
import json
import sys

from raftify.raft_client import RaftClient
from raftify.raft_utils import print_raft_node
from raftify.utils import SocketAddr


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
                # TODO: Add this
                pass
            case "debug":
                print(print_raft_node(json.loads(await client.debug_node(timeout=5.0))))
            case _:
                print_help()

    else:
        print_help()


def main():
    asyncio.run(async_main())
