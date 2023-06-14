import abc


class FSM(metaclass=abc.ABCMeta):
    @abc.abstractmethod
    async def apply(self, message: bytes) -> bytes:
        raise NotImplementedError

    @abc.abstractmethod
    async def snapshot(self) -> bytes:
        raise NotImplementedError

    @abc.abstractmethod
    async def restore(self, snapshot: bytes) -> None:
        raise NotImplementedError
