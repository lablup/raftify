import pickle
import threading
from dataclasses import dataclass
from typing import Union


class PickleSerializer:
    def encode(self):
        return pickle.dumps(self)

    @classmethod
    def decode(cls, data: bytes):
        return cls(*pickle.loads(data))


@dataclass
class SocketAddr:
    host: str
    port: int

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
