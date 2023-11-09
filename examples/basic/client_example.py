import asyncio
from raftify.state_machine.hashstore import SetCommand
from raftify.raft_client import RaftClient


async def main() -> None:
    """
    A simple set of commands to test and show usage of RaftClient.
    """

    await RaftClient("127.0.0.1:60062").propose(SetCommand("1", "A").encode())


if __name__ == "__main__":
    asyncio.run(main())
