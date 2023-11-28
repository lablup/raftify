import pickle
from typing import Any

from .abc import AbstractCodec


class PickleCodec(AbstractCodec):
    def encode(self, obj: Any) -> bytes:
        return pickle.dumps(obj)

    def decode(self, data: bytes) -> Any:
        return pickle.loads(data)
