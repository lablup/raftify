import asyncio

from raftify.raft_client import RaftClient
from raftify.state_machine.hashstore import SetCommand


async def main() -> None:
    """
    A simple set of commands to test and show usage of RaftClient.
    Please bootstrap the Raft cluster before running this script.
    """

    await RaftClient("127.0.0.1:60061").propose(SetCommand("1", "A").encode())
    await RaftClient("127.0.0.1:60062").propose(SetCommand("2", "A").encode())


if __name__ == "__main__":
    asyncio.run(main())
