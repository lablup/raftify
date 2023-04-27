import logging
import os
from threading import Lock
from typing import Any, Callable, List, Optional

import lmdb
from rraft import (
    ConfState,
    ConfState_Ref,
    Entry,
    Entry_Ref,
    GetEntriesContext,
    GetEntriesContext_Ref,
    HardState,
    HardState_Ref,
    RaftState,
    Snapshot,
    Snapshot_Ref,
)

from riteraft.utils import decode_u64, encode_u64

SNAPSHOT_KEY = b"snapshot"
LAST_INDEX_KEY = b"last_index"
HARD_STATE_KEY = b"hard_state"
CONF_STATE_KEY = b"conf_state"


class LMDBStorageCore:
    def __init__(
        self,
        env: lmdb.Environment,
        entries_db: lmdb._Database,
        metadata_db: lmdb._Database,
    ):
        self.env = env
        self.entries_db = entries_db
        self.metadata_db = metadata_db

    @staticmethod
    def create(path: os.PathLike, id: int) -> "LMDBStorageCore":
        os.makedirs(path, exist_ok=True)
        db_pth = os.path.join(path, f"raft-{id}.mdb")

        env: lmdb.Environment = lmdb.open(db_pth, map_size=100 * 4096, max_dbs=3000)
        entries_db = env.open_db(b"entries")
        metadata_db = env.open_db(b"meta")

        hard_state = HardState.default()
        conf_state = ConfState.default()

        storage = LMDBStorageCore(env, entries_db, metadata_db)

        storage.set_hard_state(hard_state)
        storage.set_conf_state(conf_state)
        storage.append([Entry.default()])

        return storage

    def hard_state(self) -> HardState:
        with self.env.begin(write=False, db=self.metadata_db) as meta_reader:
            hs = meta_reader.get(HARD_STATE_KEY)
            if hs is None:
                raise Exception("Missing hard state")
            return HardState.decode(hs)

    def set_hard_state(self, hard_state: HardState | HardState_Ref) -> None:
        with self.env.begin(write=True, db=self.metadata_db) as meta_writer:
            meta_writer.put(HARD_STATE_KEY, hard_state.encode())

    def conf_state(self) -> ConfState:
        with self.env.begin(write=False, db=self.metadata_db) as meta_reader:
            cs = meta_reader.get(CONF_STATE_KEY)
            if cs is None:
                raise Exception("There should be a conf state")
            return ConfState.decode(cs)

    def set_conf_state(self, conf_state: ConfState | ConfState_Ref) -> None:
        with self.env.begin(write=True, db=self.metadata_db) as meta_writer:
            meta_writer.put(CONF_STATE_KEY, conf_state.encode())

    def snapshot(self, _request_index: int) -> Optional[Snapshot]:
        with self.env.begin(write=False, db=self.metadata_db) as meta_reader:
            snapshot = meta_reader.get(SNAPSHOT_KEY)
            return Snapshot.decode(snapshot) if snapshot else None

    def set_snapshot(self, snapshot: Snapshot | Snapshot_Ref) -> None:
        with self.env.begin(write=True, db=self.metadata_db) as meta_writer:
            meta_writer.put(SNAPSHOT_KEY, snapshot.encode())

    def first_index(self) -> int:
        with self.env.begin(write=False, db=self.entries_db) as entry_reader:
            cursor = entry_reader.cursor()

            if not cursor.first():
                raise Exception("There should always be at least one entry in the db")

            return decode_u64(cursor.key()) + 1

    def last_index(self) -> int:
        with self.env.begin(write=False, db=self.metadata_db) as meta_reader:
            last_index = meta_reader.get(LAST_INDEX_KEY)
            return decode_u64(last_index) if last_index else 0

    def set_last_index(self, index: int) -> None:
        with self.env.begin(write=True, db=self.metadata_db) as meta_writer:
            meta_writer.put(LAST_INDEX_KEY, encode_u64(index))

    def entry(self, index: int) -> Optional[Entry]:
        with self.env.begin(write=False, db=self.entries_db) as entry_reader:
            entry = entry_reader.get(encode_u64(index))
            return Entry.decode(entry) if entry else None

    def entries(
        self,
        low: int,
        high: int,
        _ctx: GetEntriesContext | GetEntriesContext_Ref,
        max_size: Optional[int],
    ) -> List[Entry]:
        with self.env.begin(write=False, db=self.entries_db) as entry_reader:
            logging.info(f"Entries requested: {low}->{high}")

            cursor = entry_reader.cursor()
            if not cursor.set_range(encode_u64(low)):
                return []

            size_count = 0
            entries = []

            for key, entry in cursor:
                if not decode_u64(key) >= high:
                    break

                size_count += len(entry)

                if max_size and size_count >= max_size:
                    break

                entries.append(Entry.decode(entry))

            return entries

    def append(self, entries: List[Entry] | List[Entry_Ref]) -> None:
        last_index = self.last_index()

        with self.env.begin(write=True, db=self.entries_db) as entry_writer:
            # TODO: ensure entry arrive in the right order
            for entry in entries:
                # assert entry.get_index() == last_index + 1
                index = entry.get_index()
                last_index = max(index, last_index)
                entry_writer.put(encode_u64(index), entry.encode())

        self.set_last_index(last_index)

    def set_hard_state_comit(self, comit: int) -> None:
        hs = self.hard_state()
        hs.set_commit(comit)
        self.set_hard_state(hs)


class LMDBStorage:
    def __init__(self, core: LMDBStorageCore):
        self.core = core

    @staticmethod
    def create(path: os.PathLike, id: int) -> "LMDBStorage":
        core = LMDBStorageCore.create(path, id)
        return LMDBStorage(core)

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
            # TODO, check that compaction is legal
            # let last_index = self.last_index(&writer)?;
            # there should always be at least one entry in the log
            # assert!(last_index > index + 1);

            with self.core.env.begin(
                write=True, db=self.core.entries_db
            ) as entry_writer:
                cursor = entry_writer.cursor()

                for key, _ in cursor:
                    if decode_u64(key) >= index:
                        break
                    cursor.delete()

        self.wl(__compact)

    def append(self, entries: List[Entry] | List[Entry_Ref]) -> None:
        def __append(store: LMDBStorageCore):
            store.append(entries)

        self.wl(__append)

    def set_hard_state(self, hard_state: HardState | HardState_Ref) -> None:
        def __set_hard_state(store: LMDBStorageCore):
            store.set_hard_state(hard_state)

        self.wl(__set_hard_state)

    def set_conf_state(self, conf_state: ConfState | ConfState_Ref) -> None:
        def __set_conf_state(store: LMDBStorageCore):
            store.set_conf_state(conf_state)

        self.wl(__set_conf_state)

    def create_snapshot(self, data: bytes) -> None:
        def __create_snapshot(store: LMDBStorageCore):
            hard_state = store.hard_state()
            conf_state = store.conf_state()

            snapshot = Snapshot.default()
            snapshot.set_data(data)

            meta = snapshot.get_metadata()
            meta.set_conf_state(conf_state)
            meta.set_index(hard_state.get_commit())
            meta.set_term(hard_state.get_term())

            store.set_snapshot(snapshot)

        self.wl(__create_snapshot)

    def apply_snapshot(self, snapshot: Snapshot | Snapshot_Ref) -> None:
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

            logging.warning(f"Raft_state: {raft_state}")

            return raft_state

        return self.rl(__initial_state)

    def entries(
        self,
        low: int,
        high: int,
        ctx: GetEntriesContext | GetEntriesContext_Ref,
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
                raise Exception("Compacted Error")

            if idx > last_index:
                raise Exception("Unavailable Error")

            try:
                entry = store.entry(idx)
            except Exception:
                raise Exception("Unavailable Error")

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

    def snapshot(self, request_index: int) -> Optional[Snapshot]:
        def __snapshot(store: LMDBStorageCore):
            return store.snapshot(request_index)

        return self.rl(__snapshot)

    def set_hard_state_comit(self, comit: int) -> None:
        def __set_hard_state_comit(store: LMDBStorageCore):
            return store.set_hard_state_comit(comit)

        return self.wl(__set_hard_state_comit)
