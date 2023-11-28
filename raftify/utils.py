import asyncio
import os
from dataclasses import dataclass


def get_filesize(path: str) -> int:
    try:
        return os.path.getsize(path)
    except FileNotFoundError:
        return 0


@dataclass
class SocketAddr:
    host: str
    port: int

    def __repr__(self) -> str:
        return f"{self.host}:{self.port}"

    @staticmethod
    def from_str(s: str):
        ip, port = s.split(":")
        return SocketAddr(ip, int(port))


class AtomicInteger:
    def __init__(self, value: int = 0):
        self.__value = value
        self.__lock = asyncio.Lock()

    def __repr__(self) -> str:
        return str(self.__value)

    def __hash__(self) -> int:
        return self.__value

    def __eq__(self, other: object) -> bool:
        if isinstance(other, AtomicInteger):
            return self.__value == other.__value
        elif isinstance(other, int):
            return self.__value == other
        return False

    async def increase(self, value: int = 1) -> "AtomicInteger":
        async with self.__lock:
            self.__value += value
        return self

    async def decrease(self, value: int = 1) -> "AtomicInteger":
        async with self.__lock:
            self.__value -= value
        return self

    async def set(self, value: int) -> "AtomicInteger":
        async with self.__lock:
            self.__value = value
        return self

    @property
    def value(self) -> int:
        return self.__value
