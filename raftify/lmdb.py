import os
import re
from threading import Lock
from typing import List, Optional

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


def get_segment_index_from_path(path: str, node_id: int) -> int:
    files = os.listdir(path)
    pattern = re.compile(r"entry-node{}-(\d+).mdb".format(node_id))

    segment_indices = []
    for file in files:
        match = pattern.match(file)
        if match:
            segment_index = int(match.group(1))
            segment_indices.append(segment_index)

    return max(segment_indices, default=1)


class LMDBStorageCore:
    def __init__(
        self,
        storage_path: str,
        node_id: int,
        segment_index: int,
        max_segment_size: int,
        entries_env: lmdb.Environment,
        meta_env: lmdb.Environment,
        entries_db: lmdb._Database,
        metadata_db: lmdb._Database,
        logger: AbstractRaftifyLogger,
    ):
        self.storage_path = storage_path
        self.node_id = node_id
        self.segment_index = segment_index
        self.max_segment_size = max_segment_size
        self.entries_env = entries_env
        self.meta_env = meta_env
        self.entries_db = entries_db
        self.metadata_db = metadata_db
        self.logger = logger

    @classmethod
    def create_entry_db(
        cls, path: str, node_id: int, segment_index: int, map_size: int
    ) -> tuple[lmdb.Environment, lmdb._Database]:
        entry_db_pth = os.path.join(
            path, f"node{node_id}-entry.mdb", f"segment{segment_index}"
        )
        os.makedirs(entry_db_pth, exist_ok=True)
        entries_env: lmdb.Environment = lmdb.open(
            entry_db_pth, map_size=map_size, max_dbs=3000
        )
        entries_db = entries_env.open_db(b"entries")
        return entries_env, entries_db

    @classmethod
    def create_meta_db(
        cls, path: str, node_id: int, map_size: int
    ) -> tuple[lmdb.Environment, lmdb._Database]:
        meta_db_pth = os.path.join(path, f"node{node_id}-meta.mdb")
        os.makedirs(meta_db_pth, exist_ok=True)
        meta_env: lmdb.Environment = lmdb.open(
            meta_db_pth, map_size=map_size * 1000 * 1000, max_dbs=2
        )
        metadata_db = meta_env.open_db(b"meta")
        return meta_env, metadata_db

    @classmethod
    def create(
        cls,
        max_segment_size: int,
        path: str,
        node_id: int,
        logger: AbstractRaftifyLogger,
    ) -> "LMDBStorageCore":
        segment_index = get_segment_index_from_path(path, node_id)

        try:
            entries_env, entries_db = cls.create_entry_db(
                path, node_id, segment_index, max_segment_size
            )
        except lmdb.MapFullError:
            logger.error("Entry db is too small!")
            raise

        try:
            meta_env, metadata_db = cls.create_meta_db(path, node_id, 1024 * 1024)
        except lmdb.MapFullError:
            logger.error("Metadata db is full!")
            raise

        hard_state = HardState.default()
        conf_state = ConfState.default()

        core = cls(
            path,
            node_id,
            segment_index,
            max_segment_size,
            entries_env,
            meta_env,
            entries_db,
            metadata_db,
            logger,
        )

        core.set_hard_state(hard_state)
        core.set_conf_state(conf_state)
        core.append([Entry.default()])

        return core

    def create_new_segment(self) -> None:
        self.segment_index += 1
        entries_env, entries_db = LMDBStorageCore.create_entry_db(
            self.storage_path, self.node_id, self.segment_index, self.max_segment_size
        )

        self.entries_env = entries_env
        self.entries_db = entries_db

    def hard_state(self) -> HardState:
        with self.meta_env.begin(write=False, db=self.metadata_db) as meta_reader:
            hs = meta_reader.get(HARD_STATE_KEY)
            if hs is None:
                raise StoreError(UnavailableError("Missing hard_state"))
            return HardState.decode(hs)

    def set_hard_state(self, hard_state: HardState | HardStateRef) -> None:
        with self.meta_env.begin(write=True, db=self.metadata_db) as meta_writer:
            meta_writer.put(HARD_STATE_KEY, hard_state.encode())

    def conf_state(self) -> ConfState:
        with self.meta_env.begin(write=False, db=self.metadata_db) as meta_reader:
            conf_state = meta_reader.get(CONF_STATE_KEY)
            if conf_state is None:
                raise StoreError(UnavailableError("There should be a conf state"))
            return ConfState.decode(conf_state)

    def set_conf_state(self, conf_state: ConfState | ConfStateRef) -> None:
        with self.meta_env.begin(write=True, db=self.metadata_db) as meta_writer:
            meta_writer.put(CONF_STATE_KEY, conf_state.encode())

    def snapshot(self, _request_index: int, _to: int) -> Snapshot:
        with self.env.begin(write=False, db=self.metadata_db) as meta_reader:
            snapshot = meta_reader.get(SNAPSHOT_KEY)
            return Snapshot.decode(snapshot) if snapshot else Snapshot.default()

    def set_snapshot(self, snapshot: Snapshot | SnapshotRef) -> None:
        with self.meta_env.begin(write=True, db=self.metadata_db) as meta_writer:
            meta_writer.put(SNAPSHOT_KEY, snapshot.encode())

    def first_index(self) -> int:
        with self.entries_env.begin(write=False, db=self.entries_db) as entry_reader:
            cursor = entry_reader.cursor()

            if not cursor.first():
                raise StoreError(
                    UnavailableError(
                        "There should always be at least one entry in the DB"
                    )
                )

            return decode_int(cursor.key()) + 1

    def last_index(self) -> int:
        with self.meta_env.begin(write=False, db=self.metadata_db) as meta_reader:
            last_index = meta_reader.get(LAST_INDEX_KEY)
            return decode_int(last_index) if last_index else 0

    def set_last_index(self, index: int) -> None:
        with self.meta_env.begin(write=True, db=self.metadata_db) as meta_writer:
            meta_writer.put(LAST_INDEX_KEY, encode_int(index))

    def entry(self, index: int) -> Optional[Entry]:
        with self.entries_env.begin(write=False, db=self.entries_db) as entry_reader:
            entry = entry_reader.get(encode_int(index))
            return Entry.decode(entry) if entry else None

    def entries(
        self,
        from_: int,
        to: int,
        # TODO: Handle GetEntriesContextRef
        _ctx: GetEntriesContext | GetEntriesContextRef,
        max_size: Optional[int] = None,
    ) -> List[Entry]:
        with self.entries_env.begin(write=False, db=self.entries_db) as entry_reader:
            self.logger.info(f"Entries [{from_}, {to}) requested")

            cursor = entry_reader.cursor()
            if not cursor.set_range(encode_int(from_)):
                return []

            size_count = 0
            entries = []

            for key, entry in cursor:
                if decode_int(key) >= to:
                    break

                if max_size is not None and size_count >= max_size:
                    break

                entries.append(Entry.decode(entry))

                size_count += len(entry)

            return entries

    def append(self, entries: List[Entry] | List[EntryRef]) -> None:
        last_index = self.last_index()

        try:
            with self.entries_env.begin(write=True, db=self.entries_db) as entry_writer:
                # TODO: ensure entry arrive in the right order
                for entry in entries:
                    # assert entry.get_index() == last_index + 1
                    index = entry.get_index()
                    entry_writer.put(encode_int(index), entry.encode())
                    last_index = max(index, last_index)

            self.set_last_index(last_index)
        except lmdb.MapFullError:
            self.logger.error("EntryMap is full. Create new segment...")
            self.create_new_segment()
            self.append(entries)

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
        cls,
        map_size: int,
        path: str,
        node_id: int,
        logger: AbstractRaftifyLogger,
    ) -> "LMDBStorage":
        core = LMDBStorageCore.create(map_size, path, node_id, logger)
        return cls(core, logger)

    def compact(self, index: int) -> None:
        with Lock():
            with self.core.entries_env.begin(
                write=True, db=self.core.entries_db
            ) as entry_writer:
                cursor = entry_writer.cursor()
                cursor.first()

                while decode_int(cursor.key()) < index:
                    cursor.delete()

    def append(self, entries: List[Entry] | List[EntryRef]) -> None:
        with Lock():
            self.core.append(entries)

    def set_hard_state(self, hard_state: HardState | HardStateRef) -> None:
        with Lock():
            self.core.set_hard_state(hard_state)

    def set_conf_state(self, conf_state: ConfState | ConfStateRef) -> None:
        with Lock():
            self.core.set_conf_state(conf_state)

    def create_snapshot(self, data: bytes, index: int, term: int) -> None:
        with Lock():
            snapshot = Snapshot.default()
            snapshot.set_data(data)

            meta = snapshot.get_metadata()
            meta.set_conf_state(self.core.conf_state())
            meta.set_index(index)
            meta.set_term(term)

            self.core.set_snapshot(snapshot)

    def apply_snapshot(self, snapshot: Snapshot | SnapshotRef) -> None:
        with Lock():
            metadata = snapshot.get_metadata()
            hard_state = self.core.hard_state()

            hard_state.set_term(max(hard_state.get_term(), metadata.get_term()))
            hard_state.set_commit(metadata.get_index())

            self.core.set_hard_state(hard_state)
            self.core.set_conf_state(metadata.get_conf_state())
            self.core.set_last_index(metadata.get_index())
            self.core.set_snapshot(snapshot)

    def initial_state(self) -> RaftState:
        with Lock():
            raft_state = RaftState.default()
            raft_state.set_hard_state(self.core.hard_state())
            raft_state.set_conf_state(self.core.conf_state())

            self.logger.info(f"Initial RaftState: {raft_state}")

            return raft_state

    def entries(
        self,
        low: int,
        high: int,
        ctx: GetEntriesContext | GetEntriesContextRef,
        max_size: Optional[int] = None,
    ) -> List[Entry]:
        with Lock():
            return self.core.entries(low, high, ctx, max_size)

    def term(self, index: int) -> int:
        with Lock():
            first_index = self.core.first_index()
            last_index = self.core.last_index()
            hard_state = self.core.hard_state()

            if index == hard_state.get_commit():
                return hard_state.get_term()

            if index < first_index:
                raise StoreError(CompactedError())

            if index > last_index:
                raise StoreError(UnavailableError())

            try:
                entry = self.core.entry(index)
            except Exception:
                raise StoreError(UnavailableError())

            return entry.get_term() if entry else 0

    def first_index(self) -> int:
        with Lock():
            return self.core.first_index()

    def last_index(self) -> int:
        with Lock():
            return self.core.last_index()

    def snapshot(self, request_index: int, to: int) -> Snapshot:
        with Lock():
            return self.core.snapshot(request_index, to)

    def set_hard_state_comit(self, comit: int) -> None:
        with Lock():
            return self.core.set_hard_state_comit(comit)
