import asyncio
import pickle
from raftify import RaftServiceClient


class SetCommand:
    """
    Represent simple key-value command.
    Use pickle to serialize the data.
    """

    def __init__(self, key: str, value: str) -> None:
        self.key = key
        self.value = value

    def encode(self) -> bytes:
        return pickle.dumps(self.__dict__)

    @classmethod
    def decode(cls, packed: bytes) -> "SetCommand":
        unpacked = pickle.loads(packed)
        return cls(unpacked["key"], unpacked["value"])


async def main() -> None:
    """
    A simple set of commands to test and show usage of RaftServiceClient.
    Please bootstrap the Raft cluster before running this script.
    """

    client = await RaftServiceClient.build("127.0.0.1:60061")
    client.prepare_propose(SetCommand("1", "A").encode())
    await client.propose()

    peers_json = await client.get_peers()
    print("Peers: ", peers_json)


if __name__ == "__main__":
    asyncio.run(main())
