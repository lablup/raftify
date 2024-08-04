import asyncio
from contextlib import suppress
import pickle
import sys
from typing import Optional
from raftify import (
    cli_main,
    set_custom_formatters,
)


# TODO: Find a way to import these types from the superdirectory instead of copying it here.
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


def pickle_deserialize(data: bytes) -> str | None:
    if data == b"":
        return None

    if pickle.PROTO in data:
        r = pickle.loads(data[data.index(pickle.PROTO) :])
        return r

    # Not pickle data
    return None


def register_custom_deserializer() -> None:
    """
    Initialize the custom deserializers.
    """

    set_custom_formatters(
        entry_data=pickle_deserialize,
        entry_context=pickle_deserialize,
        confchange_context=pickle_deserialize,
        confchangev2_context=pickle_deserialize,
        message_context=pickle_deserialize,
        snapshot_data=pickle_deserialize,
        log_entry=pickle_deserialize,
        fsm=pickle_deserialize,
    )

class HashStore:
    """
    A simple key-value store that stores data in memory.
    Use pickle to serialize the data.
    """

    def __init__(self):
        self._store = dict()
        self._loop = asyncio.get_running_loop()

    def get(self, key: str) -> Optional[str]:
        return self._store.get(key)

    def as_dict(self) -> dict:
        return self._store

    def apply(self, msg: bytes) -> bytes:
        message = SetCommand.decode(msg)
        self._store[message.key] = message.value
        return msg

    def snapshot(self) -> bytes:
        return pickle.dumps(self._store)

    def restore(self, snapshot: bytes) -> None:
        self._store = pickle.loads(snapshot)


async def _main(argv):
    await cli_main(argv)


def main():
    register_custom_deserializer()

    argv = sys.argv
    argv.pop(0)
    argv.insert(0, "raftify_cli")

    with suppress(KeyboardInterrupt):
        asyncio.run(_main(argv))
