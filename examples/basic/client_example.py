import asyncio
import pickle

from raftify.log_entry.set_command import SetCommand
from raftify.raft_client import RaftClient


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
        "---Debug peers---",
        pickle.loads((await RaftClient("127.0.0.1:60061").get_peers()).peers),
    )


if __name__ == "__main__":
    asyncio.run(main())
