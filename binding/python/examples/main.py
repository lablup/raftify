import asyncio
import pickle
from typing import Optional
from raftify import Raft, Config


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


class HashStore:
    """
    A simple key-value store that stores data in memory.
    Use pickle to serialize the data.
    """

    def __init__(self):
        self._store = dict()

    def get(self, key: str) -> Optional[str]:
        return self._store.get(key)

    def as_dict(self) -> dict:
        return self._store

    async def apply(self, msg: bytes) -> bytes:
        message = SetCommand.decode(msg)
        self._store[message.key] = message.value
        return msg

    async def snapshot(self) -> bytes:
        return pickle.dumps(self._store)

    async def restore(self, snapshot: bytes) -> None:
        self._store = pickle.loads(snapshot)


# # * Thread version
# def run_raft():
#     cfg = Config()
#     addr = "127.0.0.1:60161"
#     store = HashStore()
#     raft = Raft.build(1, addr, store, cfg)
#     raft.run()


# Threading
# thread = threading.Thread(target=run_raft)
# thread.start()

# Coroutine
async def main():
    cfg = Config()
    addr = "127.0.0.1:60161"
    store = HashStore()
    raft = Raft.build(1, addr, store, cfg)
    task = asyncio.create_task(raft.run())

    await asyncio.gather(task)

if __name__ == "__main__":
    asyncio.run(main())
