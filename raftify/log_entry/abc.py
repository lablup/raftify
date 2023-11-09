import abc


class AbstractLogEntry(metaclass=abc.ABCMeta):
    @abc.abstractmethod
    def encode(self) -> bytes:
        raise NotImplementedError

    @classmethod
    def decode(cls, packed: bytes) -> "AbstractLogEntry":
        raise NotImplementedError
