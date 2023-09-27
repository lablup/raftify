import os
import sys
import lmdb
import rraft
from raftify.deserializer import init_rraft_py_deserializer
from raftify.lmdb import SNAPSHOT_KEY, LAST_INDEX_KEY, HARD_STATE_KEY, CONF_STATE_KEY


def main(argv):
    init_rraft_py_deserializer()
    node_id = argv[1]
    segment_idx = argv[2]
    assert node_id.isdigit(), "node_id must be a number"
    assert segment_idx.isdigit(), "segment_idx must be a number"

    entries_env = lmdb.open(f"{os.getcwd()}/node{node_id}-entry.mdb/segment{segment_idx}", max_dbs=2)
    entries_db = entries_env.open_db(b"entries")

    print('---- Entries ----')
    with entries_env.begin(db=entries_db) as txn:
        cursor = txn.cursor()
        for key, value in cursor:
            print(f"Key: {int(key.decode())}, Value: {rraft.Entry.decode(value)}")

    metadata_env = lmdb.open(f"{os.getcwd()}/node{node_id}-meta.mdb", max_dbs=2)
    metadata_db = metadata_env.open_db(b"meta")

    print('---- Metadata ----')
    with metadata_env.begin(db=metadata_db) as txn:
        cursor = txn.cursor()
        for key, value in cursor:
            if key == SNAPSHOT_KEY:
                print(f'Key: "snapshot", Value: "{rraft.Snapshot.decode(value)}"')
            elif key == LAST_INDEX_KEY:
                print(f'Key: "last_index", Value: "{int(value.decode())}"')
            elif key == HARD_STATE_KEY:
                print(f'Key: "hard_state", Value: "{rraft.HardState.decode(value)}"')
            elif key == CONF_STATE_KEY:
                print(f'Key: "conf_state", Value: "{rraft.ConfState.decode(value)}"')
            else:
                assert False, f"Unknown key: {key}"

    metadata_env.close()


if __name__ == "__main__":
    main(sys.argv)
