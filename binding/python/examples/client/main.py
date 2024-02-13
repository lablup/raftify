import asyncio
from raftify import RaftServiceClient

from ..state_machine import SetCommand


# Run "python -m examples.client.main" to execute this script.
async def main() -> None:
    """
    A simple set of commands to test and show usage of RaftServiceClient.
    Please bootstrap the Raft cluster before running this script.
    """

    client = await RaftServiceClient.build("127.0.0.1:60061")
    await client.propose(SetCommand("1", "A").encode())

    peers_json = await client.get_peers()
    print("Peers: ", peers_json)


if __name__ == "__main__":
    asyncio.run(main())
