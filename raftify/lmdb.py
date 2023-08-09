import os
from threading import Lock
from typing import Any, Callable, List, Optional

import lmdb
from rraft import (
    CompactedError,
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
    StoreError,
    UnavailableError,
)

from raftify.logger import AbstractRaftifyLogger

SNAPSHOT_KEY = b"snapshot"
LAST_INDEX_KEY = b"last_index"
HARD_STATE_KEY = b"hard_state"
CONF_STATE_KEY = b"conf_state"


def encode_int(v: int) -> bytes:
    # TODO: Add exception handling logic when v is greater than 8 digits
    assert v < 10**8, "Value greater than 10 ** 8 not supported"
    return str(v).zfill(8).encode()


def decode_int(v: bytes) -> int:
    return int(v.decode())


class LMDBStorageCore:
    def __init__(
        self,
        env: lmdb.Environment,
        entries_db: lmdb._Database,
        metadata_db: lmdb._Database,
        logger: AbstractRaftifyLogger,
    ):
        self.env = env
        self.entries_db = entries_db
        self.metadata_db = metadata_db
        self.logger = logger

    @classmethod
    def create(
        cls, path: os.PathLike, id: int, logger: AbstractRaftifyLogger
    ) -> "LMDBStorageCore":
        os.makedirs(path, exist_ok=True)
        db_pth = os.path.join(path, f"raft-{id}.mdb")

        env: lmdb.Environment = lmdb.open(db_pth, map_size=100 * 4096, max_dbs=3000)
        entries_db = env.open_db(b"entries")
        metadata_db = env.open_db(b"meta")

        hard_state = HardState.default()
        conf_state = ConfState.default()

        core = cls(env, entries_db, metadata_db, logger)

        core.set_hard_state(hard_state)
        core.set_conf_state(conf_state)
        core.append([Entry.default()])

        return core

    def hard_state(self) -> HardState:
        with self.env.begin(write=False, db=self.metadata_db) as meta_reader:
            hs = meta_reader.get(HARD_STATE_KEY)
            if hs is None:
                raise StoreError(UnavailableError("Missing hard_state"))
            return HardState.decode(hs)

    def set_hard_state(self, hard_state: HardState | HardStateRef) -> None:
        with self.env.begin(write=True, db=self.metadata_db) as meta_writer:
            meta_writer.put(HARD_STATE_KEY, hard_state.encode())

    def conf_state(self) -> ConfState:
        with self.env.begin(write=False, db=self.metadata_db) as meta_reader:
            cs = meta_reader.get(CONF_STATE_KEY)
            if cs is None:
                raise StoreError(UnavailableError("There should be a conf state"))
            return ConfState.decode(cs)

    def set_conf_state(self, conf_state: ConfState | ConfStateRef) -> None:
        with self.env.begin(write=True, db=self.metadata_db) as meta_writer:
            meta_writer.put(CONF_STATE_KEY, conf_state.encode())

    def snapshot(self, _request_index: int, _to: int) -> Optional[Snapshot]:
        with self.env.begin(write=False, db=self.metadata_db) as meta_reader:
            snapshot = meta_reader.get(SNAPSHOT_KEY)
            return Snapshot.decode(snapshot) if snapshot else None

    def set_snapshot(self, snapshot: Snapshot | SnapshotRef) -> None:
        with self.env.begin(write=True, db=self.metadata_db) as meta_writer:
            meta_writer.put(SNAPSHOT_KEY, snapshot.encode())

    def first_index(self) -> int:
        with self.env.begin(write=False, db=self.entries_db) as entry_reader:
            cursor = entry_reader.cursor()

            if not cursor.first():
                raise StoreError(
                    UnavailableError(
                        "There should always be at least one entry in the DB"
                    )
                )

            return decode_int(cursor.key()) + 1

    def last_index(self) -> int:
        with self.env.begin(write=False, db=self.metadata_db) as meta_reader:
            last_index = meta_reader.get(LAST_INDEX_KEY)
            return decode_int(last_index) if last_index else 0

    def set_last_index(self, index: int) -> None:
        with self.env.begin(write=True, db=self.metadata_db) as meta_writer:
            meta_writer.put(LAST_INDEX_KEY, encode_int(index))

    def entry(self, index: int) -> Optional[Entry]:
        with self.env.begin(write=False, db=self.entries_db) as entry_reader:
            entry = entry_reader.get(encode_int(index))
            return Entry.decode(entry) if entry else None

    def entries(
        self,
        low: int,
        high: int,
        _ctx: GetEntriesContext | GetEntriesContextRef,
        max_size: Optional[int] = None,
    ) -> List[Entry]:
        with self.env.begin(write=False, db=self.entries_db) as entry_reader:
            self.logger.info(f"Entries [{low}, {high}) requested")

            cursor = entry_reader.cursor()
            if not cursor.set_range(encode_int(low)):
                return []

            size_count = 0
            entries = []

            for key, entry in cursor:
                if decode_int(key) >= high:
                    break

                if max_size is not None and size_count >= max_size:
                    break

                entries.append(Entry.decode(entry))

                size_count += len(entry)

            return entries

    def append(self, entries: List[Entry] | List[EntryRef]) -> None:
        last_index = self.last_index()

        with self.env.begin(write=True, db=self.entries_db) as entry_writer:
            # TODO: ensure entry arrive in the right order
            for entry in entries:
                # assert entry.get_index() == last_index + 1
                index = entry.get_index()
                entry_writer.put(encode_int(index), entry.encode())
                last_index = max(index, last_index)

        self.set_last_index(last_index)

    def set_hard_state_comit(self, comit: int) -> None:
        hs = self.hard_state()
        hs.set_commit(comit)
        self.set_hard_state(hs)


class LMDBStorage:
    def __init__(self, core: LMDBStorageCore, logger: AbstractRaftifyLogger):
        self.core = core
        self.logger = logger

    @classmethod
    def create(
        cls, path: os.PathLike, id: int, logger: AbstractRaftifyLogger
    ) -> "LMDBStorage":
        core = LMDBStorageCore.create(path, id, logger)
        return cls(core, logger)

    # TODO: Refactor below logic.
    def wl(self, cb: Callable[[LMDBStorageCore], Any]) -> Any:
        with Lock():
            res = cb(self.core)
        return res

    # TODO: Refactor below logic.
    def rl(self, cb: Callable[[LMDBStorageCore], Any]) -> Any:
        with Lock():
            res = cb(self.core)
        return res

    def compact(self, index: int) -> None:
        def __compact(_store: LMDBStorageCore):
            # TODO: check that compaction is legal
            # let last_index = self.last_index(&writer)?;
            # there should always be at least one entry in the log
            # assert!(last_index > index + 1);

            with self.core.env.begin(
                write=True, db=self.core.entries_db
            ) as entry_writer:
                cursor = entry_writer.cursor()
                cursor.first()

                while decode_int(cursor.key()) < index:
                    cursor.delete()

        self.wl(__compact)

    def append(self, entries: List[Entry] | List[EntryRef]) -> None:
        def __append(store: LMDBStorageCore):
            store.append(entries)

        self.wl(__append)

    def set_hard_state(self, hard_state: HardState | HardStateRef) -> None:
        def __set_hard_state(store: LMDBStorageCore):
            store.set_hard_state(hard_state)

        self.wl(__set_hard_state)

    def set_conf_state(self, conf_state: ConfState | ConfStateRef) -> None:
        def __set_conf_state(store: LMDBStorageCore):
            store.set_conf_state(conf_state)

        self.wl(__set_conf_state)

    def create_snapshot(self, data: bytes, index: int, term: int) -> None:
        def __create_snapshot(store: LMDBStorageCore):
            conf_state = store.conf_state()

            snapshot = Snapshot.default()
            snapshot.set_data(data)

            meta = snapshot.get_metadata()
            meta.set_conf_state(conf_state)
            meta.set_index(index)
            meta.set_term(term)

            store.set_snapshot(snapshot)

        self.wl(__create_snapshot)

    def apply_snapshot(self, snapshot: Snapshot | SnapshotRef) -> None:
        def __apply_snapshot(store: LMDBStorageCore):
            metadata = snapshot.get_metadata()
            conf_state = metadata.get_conf_state()
            hard_state = store.hard_state()

            hard_state.set_term(max(hard_state.get_term(), metadata.get_term()))
            hard_state.set_commit(metadata.get_index())

            store.set_hard_state(hard_state)
            store.set_conf_state(conf_state)
            store.set_last_index(metadata.get_index())

        self.wl(__apply_snapshot)

    def initial_state(self) -> RaftState:
        def __initial_state(store: LMDBStorageCore):
            raft_state = RaftState.default()
            raft_state.set_hard_state(store.hard_state())
            raft_state.set_conf_state(store.conf_state())

            self.logger.info(f"Initial RaftState: {raft_state}")

            return raft_state

        return self.rl(__initial_state)

    def entries(
        self,
        low: int,
        high: int,
        ctx: GetEntriesContext | GetEntriesContextRef,
        max_size: Optional[int] = None,
    ) -> List[Entry]:
        def __entries(store: LMDBStorageCore):
            return store.entries(low, high, ctx, max_size)

        return self.rl(__entries)

    def term(self, idx: int) -> int:
        def __term(store: LMDBStorageCore):
            first_index = store.first_index()
            last_index = store.last_index()
            hard_state = store.hard_state()

            if idx == hard_state.get_commit():
                return hard_state.get_term()

            if idx < first_index:
                raise StoreError(CompactedError())

            if idx > last_index:
                raise StoreError(UnavailableError())

            try:
                entry = store.entry(idx)
            except Exception:
                raise StoreError(UnavailableError())

            return entry.get_term() if entry else 0

        return self.rl(__term)

    def first_index(self) -> int:
        def __first_index(store: LMDBStorageCore):
            return store.first_index()

        return self.rl(__first_index)

    def last_index(self) -> int:
        def __last_index(store: LMDBStorageCore):
            return store.last_index()

        return self.rl(__last_index)

    def snapshot(self, request_index: int, to: int) -> Optional[Snapshot]:
        def __snapshot(store: LMDBStorageCore):
            return store.snapshot(request_index, to)

        return self.rl(__snapshot)

    def set_hard_state_comit(self, comit: int) -> None:
        def __set_hard_state_comit(store: LMDBStorageCore):
            return store.set_hard_state_comit(comit)

        return self.wl(__set_hard_state_comit)
