import abc


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
