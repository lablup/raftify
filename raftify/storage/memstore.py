from typing import List, Optional

import rraft
from rraft import (
    ConfState,
    ConfStateRef,
    Entry,
    EntryRef,
    GetEntriesContext,
    GetEntriesContextRef,
    HardState,
    HardStateRef,
    RaftState,
    Snapshot,
    SnapshotRef,
)

from ..logger import AbstractRaftifyLogger

SNAPSHOT_KEY = b"snapshot"
LAST_INDEX_KEY = b"last_index"
HARD_STATE_KEY = b"hard_state"
CONF_STATE_KEY = b"conf_state"


class MemStorage:
    def __init__(
        self,
        storage: rraft.MemStorage,
        _snapshot: rraft.Snapshot,
        logger: AbstractRaftifyLogger,
    ):
        self.storage = storage
        self._snapshot = _snapshot
        self.logger = logger

    @classmethod
    def create(
        cls,
        logger: AbstractRaftifyLogger,
    ) -> "MemStorage":
        storage = rraft.MemStorage.default()
        snapshot = rraft.Snapshot.default()
        return cls(storage, snapshot, logger)

    def hard_state(self) -> HardState:
        return self.storage.wl().hard_state()

    def set_hard_state(self, hard_state: HardState | HardStateRef) -> None:
        self.storage.wl().set_hardstate(hard_state)

    def conf_state(self) -> ConfState:
        raise NotImplementedError

    def set_conf_state(self, conf_state: ConfState | ConfStateRef) -> None:
        self.storage.wl().set_conf_state(conf_state)

    def snapshot(self, request_index: int, to: int) -> Snapshot:
        return self.storage.snapshot(request_index, to)

    def create_snapshot(self, data: bytes, request_index: int, to: int) -> None:
        self._snapshot = self.storage.snapshot(request_index, to)
        self._snapshot.set_data(data)

    def apply_snapshot(self, snapshot: Snapshot | SnapshotRef) -> None:
        self.storage.wl().apply_snapshot(snapshot)

    def first_index(self) -> int:
        return self.storage.first_index()

    def last_index(self) -> int:
        return self.storage.last_index()

    def entries(
        self,
        from_: int,
        to: int,
        ctx: GetEntriesContext | GetEntriesContextRef,
        max_size: Optional[int] = None,
    ) -> List[Entry]:
        return self.storage.entries(from_, to, ctx, max_size)

    def compact(self, to: int) -> None:
        self.storage.wl().compact(to)

    def append(self, entries: List[Entry] | List[EntryRef]) -> None:
        self.storage.wl().append(entries)

    def set_hard_state_comit(self, comit: int) -> None:
        hs = self.hard_state().clone()
        hs.set_commit(comit)
        self.storage.wl().set_hardstate(hs)

    def initial_state(self) -> RaftState:
        return self.storage.initial_state()

    def term(self, index: int) -> int:
        return self.storage.term(index)
