import asyncio

from raftify.raft_client import RaftClient
from raftify.state_machine.hashstore import SetCommand


async def main() -> None:
    """
    A simple set of commands to test and show usage of RaftClient.
    Please bootstrap the Raft cluster before running this script.
    """

    print("---Message propose---")
    await RaftClient("127.0.0.1:60061").propose(SetCommand("1", "A").encode())

    print("---Message propose rerouting---")
    await RaftClient("127.0.0.1:60062").propose(SetCommand("2", "A").encode())

    print("---Debug node result---", await RaftClient("127.0.0.1:60061").debug_node())

    print(
        "---Debug entries result---",
        await RaftClient("127.0.0.1:60061").debug_entries(),
    )


if __name__ == "__main__":
    asyncio.run(main())
