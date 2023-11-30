import pickle
from typing import Any, Optional

from .abc import AbstractCodec


class PickleCodec(AbstractCodec):
    def encode(self, obj: Any) -> bytes:
        return pickle.dumps(obj)

    def decode(self, data: Optional[bytes]) -> Optional[Any]:
        if not data:
            return None
        return pickle.loads(data)
