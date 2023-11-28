import os
import shutil
from typing import Optional

import lmdb
import rraft
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

from raftify.raft_utils import append_to_json_file
from raftify.utils import get_filesize

from ..logger import AbstractRaftifyLogger
from ..rraft_deserializer import entry_data_deserializer, pickle_deserialize

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


class LMDBStorage:
    def __init__(
        self,
        log_dir_path: str,
        compacted_log_dir_path: str,
        compacted_logs_size_threshold: int,
        node_id: int,
        env: lmdb.Environment,
        entries_db: lmdb._Database,
        metadata_db: lmdb._Database,
        logger: AbstractRaftifyLogger,
    ):
        self.node_id = node_id
        self.env = env
        self.entries_db = entries_db
        self.metadata_db = metadata_db
        self.logger = logger
        self.log_dir_path = log_dir_path
        self.compacted_log_dir_path = compacted_log_dir_path
        self.compacted_logs_size_threshold = compacted_logs_size_threshold

    @classmethod
    def create(
        cls,
        map_size: int,
        log_dir_path: str,
        compacted_log_dir_path: str,
        compacted_logs_size_threshold: int,
        cluster_id: str,
        node_id: int,
        logger: AbstractRaftifyLogger,
    ) -> "LMDBStorage":
        log_dir_path = os.path.join(log_dir_path, cluster_id, f"node-{node_id}")

        compacted_log_dir_path = os.path.join(
            compacted_log_dir_path, cluster_id, f"node-{node_id}"
        )

        if os.path.exists(log_dir_path):
            shutil.rmtree(log_dir_path, ignore_errors=True)

        if os.path.exists(compacted_log_dir_path):
            shutil.rmtree(compacted_log_dir_path, ignore_errors=True)

        os.makedirs(log_dir_path, exist_ok=True)
        os.makedirs(compacted_log_dir_path, exist_ok=True)

        try:
            env: lmdb.Environment = lmdb.open(
                log_dir_path, map_size=map_size, max_dbs=3000
            )
            entries_db = env.open_db(b"entries")
            metadata_db = env.open_db(b"meta")
        except lmdb.MapFullError:
            logger.error("MDB mapsize is too small. Increase mapsize and try again.")
            raise

        hard_state = HardState.default()
        conf_state = ConfState.default()

        core = cls(
            log_dir_path,
            compacted_log_dir_path,
            compacted_logs_size_threshold,
            node_id,
            env,
            entries_db,
            metadata_db,
            logger,
        )
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

    # TODO: Handle _request_index and _to
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
    ) -> list[Entry]:
        with self.env.begin(write=False, db=self.entries_db) as entry_reader:
            self.logger.info(f"Entries [{from_}, {to}) requested")

            cursor = entry_reader.cursor()
            if not cursor.set_range(encode_int(from_)):
                return []

            size = 0
            entries = []

            for key, entry in cursor:
                if decode_int(key) >= to:
                    break

                if max_size is not None and size >= max_size:
                    # Note that even in cases where entry's size exceeds the max_size,
                    # `entries` must return at least one entry if possible.
                    if not entries:
                        return [Entry.decode(entry)]
                    return entries

                entries.append(Entry.decode(entry))
                size += len(entry)

            return entries

    def all_entries(
        self,
        max_size: Optional[int] = None,
    ) -> list[Entry]:
        with self.env.begin(write=False, db=self.entries_db) as entry_reader:
            cursor = entry_reader.cursor()

            size = 0
            entries = []

            for key, entry in cursor:
                if max_size is not None and size >= max_size:
                    break
                entries.append(Entry.decode(entry))
                size += len(entry)

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

            compaction_logs = []
            while cursor.key() and decode_int(cursor.key()) < to:
                entry = rraft.Entry.decode(cursor.value())
                entry_dict = entry.to_dict()
                entry_dict["data"] = entry_data_deserializer(entry_dict["data"])
                entry_dict["context"] = pickle_deserialize(entry_dict["context"])

                compaction_logs.append(entry_dict)

                # TODO: Maybe it would be better to include 'last_index' in compact, but when all entries removed from lmdb,
                # performance issue occurred. (and also see https://github.com/lablup/raftify/issues/47)
                # So, keep the last index entry here.

                if not cursor.delete():
                    self.logger.info(
                        f"Try to delete item at {decode_int(cursor.key())}, but not exist!"
                    )

            compacted_log_dir_path = os.path.join(
                self.compacted_log_dir_path,
                "compacted-logs.json",
            )

            if (
                get_filesize(compacted_log_dir_path)
                > self.compacted_logs_size_threshold
            ):
                self.logger.info(
                    f"Compacted log size is over {self.compacted_logs_size_threshold}. Removing all previous compacted logs."
                )
                os.remove(compacted_log_dir_path)

            append_to_json_file(
                compacted_log_dir_path,
                compaction_logs,
            )

            if to > from_:
                self.logger.info(f"Entries [{from_}, {to}) deleted successfully.")

    def append(self, entries: list[Entry] | list[EntryRef]) -> None:
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
            self.logger.info(
                "MDB is full. Clearing previous log entries and trying to append again."
            )
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
        snapshot = self.snapshot(0, 0)
        if snapshot.get_metadata().get_index() == index:
            return snapshot.get_metadata().get_term()

        try:
            entry = self.entry(index)
        except StoreError as e:
            self.logger.error(f"Unknown error occurred while getting entry. Err: {e}")
            raise StoreError(UnavailableError())

        if not entry:
            if index < self.first_index():
                raise StoreError(CompactedError())
            else:
                raise StoreError(UnavailableError())

        return entry.get_term()
