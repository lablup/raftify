import logging
import pickle
from typing import Optional

import raftify
from tests.harness.log import SetCommand


class HashStore(raftify.FSM):
    def __init__(self):
        self._store = dict()

    def get(self, key: int) -> Optional[str]:
        return self._store.get(key)

    def as_dict(self) -> dict:
        return self._store

    async def apply(self, msg: bytes) -> bytes:
        message = SetCommand.decode(msg)
        self._store[message.key] = message.value
        logging.info(f'SetCommand inserted: ({message.key}, "{message.value}")')
        return pickle.dumps(message.value)

    async def snapshot(self) -> bytes:
        return pickle.dumps(self._store)

    async def restore(self, snapshot: bytes) -> None:
        self._store = pickle.loads(snapshot)
