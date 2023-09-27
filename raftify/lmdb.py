import os
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
        cls, map_size: int, path: str, id: int, logger: AbstractRaftifyLogger
    ) -> "LMDBStorageCore":
        os.makedirs(path, exist_ok=True)
        db_pth = os.path.join(path, f"raft-{id}.mdb")

        try:
            env: lmdb.Environment = lmdb.open(db_pth, map_size=map_size, max_dbs=3000)
            entries_db = env.open_db(b"entries")
            metadata_db = env.open_db(b"meta")
        except lmdb.MapFullError:
            logger.error("MDB mapsize is too small. Increase mapsize and try again.")
            raise

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
                raise StoreError(UnavailableError("Missing hard_state in metadata"))
            return HardState.decode(hs)

    def __metadata_put(self, key: bytes, value: bytes) -> None:
        try:
            with self.env.begin(write=True, db=self.metadata_db) as meta_writer:
                meta_writer.put(key, value)
        except lmdb.MapFullError:
            self.logger.info("MDB is full. Clearing previous logs and trying again.")
            self.compact(self.last_index() - 1)
            with self.env.begin(write=True, db=self.metadata_db) as meta_writer:
                meta_writer.put(key, value)

    def set_hard_state(self, hard_state: HardState | HardStateRef) -> None:
        self.__metadata_put(HARD_STATE_KEY, hard_state.encode())

    def conf_state(self) -> ConfState:
        with self.env.begin(write=False, db=self.metadata_db) as meta_reader:
            conf_state = meta_reader.get(CONF_STATE_KEY)
            if conf_state is None:
                raise StoreError(UnavailableError("Missing conf_state in metadata"))
            return ConfState.decode(conf_state)

    def set_conf_state(self, conf_state: ConfState | ConfStateRef) -> None:
        self.__metadata_put(CONF_STATE_KEY, conf_state.encode())

    def snapshot(self, _request_index: int, _to: int) -> Snapshot:
        with self.env.begin(write=False, db=self.metadata_db) as meta_reader:
            snapshot = meta_reader.get(SNAPSHOT_KEY)
            return Snapshot.decode(snapshot) if snapshot else Snapshot.default()

    def set_snapshot(self, snapshot: Snapshot | SnapshotRef) -> None:
        self.__metadata_put(SNAPSHOT_KEY, snapshot.encode())

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
        self.__metadata_put(LAST_INDEX_KEY, encode_int(index))

    def entry(self, index: int) -> Optional[Entry]:
        with self.env.begin(write=False, db=self.entries_db) as entry_reader:
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
        with self.env.begin(write=False, db=self.entries_db) as entry_reader:
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

    def create_snapshot(self, data: bytes, index: int, term: int) -> None:
        snapshot = Snapshot.default()
        snapshot.set_data(data)

        meta = snapshot.get_metadata()
        meta.set_conf_state(self.conf_state())
        meta.set_index(index)
        meta.set_term(term)

        self.set_snapshot(snapshot)

    def compact(self, to: int) -> None:
        with self.env.begin(write=True, db=self.entries_db) as entry_writer:
            cursor = entry_writer.cursor()
            assert cursor.first(), "DB Empty!"
            from_ = decode_int(cursor.key())

            # TODO: Maybe it would be better to include 'last_index' in compact, but when all entries removed from lmdb,
            # performance issue occurred. So, keep the last index entry here.
            while cursor.key() and decode_int(cursor.key()) < to:
                if not cursor.delete():
                    self.logger.info(
                        f"Try to delete item at {decode_int(cursor.key())}, but not exist!"
                    )

            if to > from_:
                self.logger.info(f"Entries [{from_}, {to}) deleted successfully.")

    def append(self, entries: List[Entry] | List[EntryRef]) -> None:
        last_index = self.last_index()

        try:
            with self.env.begin(write=True, db=self.entries_db) as entry_writer:
                # TODO: ensure entry arrive in the right order
                for entry in entries:
                    # assert entry.get_index() == last_index + 1
                    index = entry.get_index()
                    entry_writer.put(encode_int(index), entry.encode())
                    last_index = max(index, last_index)

            self.set_last_index(last_index)
        except lmdb.MapFullError:
            self.logger.info("MDB is full. Clearing previous logs and trying again.")
            self.compact(last_index - 1)
            self.append(entries)

    def set_hard_state_comit(self, comit: int) -> None:
        hs = self.hard_state()
        hs.set_commit(comit)
        self.set_hard_state(hs)

    def apply_snapshot(self, snapshot: Snapshot | SnapshotRef) -> None:
        metadata = snapshot.get_metadata()
        hard_state = self.hard_state()

        hard_state.set_term(max(hard_state.get_term(), metadata.get_term()))
        hard_state.set_commit(metadata.get_index())

        self.set_hard_state(hard_state)
        self.set_conf_state(metadata.get_conf_state())
        self.set_last_index(metadata.get_index())
        self.set_snapshot(snapshot)

    def initial_state(self) -> RaftState:
        raft_state = RaftState.default()
        raft_state.set_hard_state(self.hard_state())
        raft_state.set_conf_state(self.conf_state())

        self.logger.info(f"Initial RaftState: {raft_state}")

        return raft_state

    def term(self, index: int) -> int:
        with Lock():
            first_index = self.first_index()
            last_index = self.last_index()
            hard_state = self.hard_state()

            if index == hard_state.get_commit():
                return hard_state.get_term()

            if index < first_index:
                raise StoreError(CompactedError())

            if index > last_index:
                raise StoreError(UnavailableError())

            try:
                entry = self.entry(index)
            except Exception:
                raise StoreError(UnavailableError())

            return entry.get_term() if entry else 0


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
            self.core.compact(index)

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
            self.core.create_snapshot(data, index, term)

    def apply_snapshot(self, snapshot: Snapshot | SnapshotRef) -> None:
        with Lock():
            self.core.apply_snapshot(snapshot)

    def initial_state(self) -> RaftState:
        with Lock():
            return self.core.initial_state()

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
            return self.core.term(index)

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
