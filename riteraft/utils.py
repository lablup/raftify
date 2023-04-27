import threading
from typing import Union


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


def encode_u64(v: int) -> bytes:
    # TODO:: Add logic for checking value of v is fitted within the range of u64.
    return v.to_bytes(8, byteorder="little", signed=True)


def decode_u64(v: bytes) -> int:
    return int.from_bytes(v, "little", signed=True)
