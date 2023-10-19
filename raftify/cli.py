import asyncio
import json
import sys

from raftify.raft_client import RaftClient
from raftify.raft_utils import print_raft_node
from raftify.utils import SocketAddr


async def async_main():
    if len(sys.argv) > 2 and sys.argv[1] == "debug":
        addr = SocketAddr.from_str(sys.argv[2])
        client = RaftClient(addr)
        print(print_raft_node(json.loads(await client.debug_node(timeout=5.0))))

    else:
        print(
            "Usage:\n"
            "  raftify-cli debug <ip:port> - Inspect a raft node at the specified IP address and port.\n"
            "Examples:\n"
            "  raftify-cli debug 127.0.0.1:60061"
        )


def main():
    asyncio.run(async_main())

