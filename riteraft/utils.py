import threading
from typing import Type, TypeVar, Union


class SocketAddr:
    def __init__(self, host: str, port: str):
        self.host = host
        self.port = port

    def __repr__(self) -> str:
        return f"{self.host}:{self.port}"

    @staticmethod
    def from_str(s: str):
        return SocketAddr(*s.split(":"))


class AtomicInteger:
    def __init__(self, value: int = 0):
        self.__value = value
        self.__lock = threading.Lock()

    def __repr__(self) -> str:
        return str(self.__value)

    def __hash__(self) -> int:
        return self.__value

    def __eq__(self, other: Union["AtomicInteger", int]) -> bool:
        if isinstance(other, AtomicInteger):
            return self.__value == other.__value
        elif isinstance(other, int):
            return self.__value == other
        return False

    def increase(self, value: int = 1) -> "AtomicInteger":
        with self.__lock:
            self.__value += value
        return self

    def decrease(self, value: int = 1) -> "AtomicInteger":
        with self.__lock:
            self.__value -= value
        return self

    def set(self, value: int) -> "AtomicInteger":
        with self.__lock:
            self.__value = value
        return self

    @property
    def value(self) -> int:
        return self.__value


T_aobj = TypeVar("T_aobj", bound="aobject")


class aobject(object):
    """
    An "asynchronous" object which guarantees to invoke both ``def __init__(self, ...)`` and
    ``async def __ainit(self)__`` to ensure asynchronous initialization of the object.

    You can create an instance of subclasses of aboject in the following way:

    .. code-block:: python

       o = await SomeAObj.new(...)
    """

    @classmethod
    async def new(cls: Type[T_aobj], *args, **kwargs) -> T_aobj:
        """
        We can do ``await SomeAObject(...)``, but this makes mypy
        to complain about its return type with ``await`` statement.
        This is a copy of ``__new__()`` to workaround it.
        """
        instance = super().__new__(cls)
        cls.__init__(instance, *args, **kwargs)
        await instance.__ainit__()
        return instance

    def __init__(self, *args, **kwargs):
        pass

    async def __ainit__(self) -> None:
        """
        Automatically called when creating the instance using
        ``await SubclassOfAObject(...)``
        where the arguments are passed to ``__init__()`` as in
        the vanilla Python classes.
        """
        pass


def encode_u64(v: int) -> bytes:
    # TODO:: Add logic for checking value of v is fitted within the range of u64.
    return v.to_bytes(8, byteorder="little", signed=True)


def decode_u64(v: bytes) -> int:
    return int.from_bytes(v, "little", signed=True)
