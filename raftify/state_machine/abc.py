import abc


class AbstractLogEntry(metaclass=abc.ABCMeta):
    @abc.abstractmethod
    def encode(self) -> bytes:
        raise NotImplementedError

    @classmethod
    def decode(cls, packed: bytes) -> "AbstractLogEntry":
        raise NotImplementedError


class AbstractStateMachine(metaclass=abc.ABCMeta):
    """
    A Finite State Machine (FSM) class.
    This class is designed to apply commands to a state, take snapshots of the state,
    and restore the state from a snapshot.
    """

    @abc.abstractmethod
    async def apply(self, message: bytes) -> bytes:
        raise NotImplementedError

    @abc.abstractmethod
    async def snapshot(self) -> bytes:
        raise NotImplementedError

    @abc.abstractmethod
    async def restore(self, snapshot: bytes) -> None:
        raise NotImplementedError
