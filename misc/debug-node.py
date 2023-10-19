import asyncio
from raftify.raft_client import RaftClient
from raftify.utils import SocketAddr


# Simple example code for debugging raft-node through gRPC server.
async def main():
    addr: SocketAddr = SocketAddr("127.0.0.1", 60061)
    client = RaftClient(addr)

    print(await client.debug_node(timeout=5.0))


asyncio.run(main())
