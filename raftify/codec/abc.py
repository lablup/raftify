from abc import ABCMeta, abstractmethod
from typing import Any


class AbstractCodec(metaclass=ABCMeta):
    @abstractmethod
    def encode(self, obj: Any) -> bytes:
        raise NotImplementedError

    @abstractmethod
    def decode(self, data: bytes) -> Any:
        raise NotImplementedError
