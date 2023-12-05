import os
import sys
import lmdb
import rraft
from raftify.config import DEFAULT_CLUSTER_ID
from raftify.rraft_deserializer import init_rraft_py_deserializer
from raftify.storage.lmdb import SNAPSHOT_KEY, LAST_INDEX_KEY, HARD_STATE_KEY, CONF_STATE_KEY


def main(argv):
    """
    Print all persisted entries.
    If the RaftNode server is running, instead of using this script, use `raftify-cli debug node 127.0.0.1:60061` for better debugging experience.
    """

    init_rraft_py_deserializer()
    idx = argv[1]
    assert idx.isdigit(), "idx must be a number"

    env = lmdb.open(f"{os.getcwd()}/raft-{idx}", max_dbs=2)

    entries_db = env.open_db(b"entries")

    print('---- Entries ----')
    with env.begin(db=entries_db) as txn:
        cursor = txn.cursor()
        for key, value in cursor:
            print(f"Key: {int(key.decode())}, Value: {rraft.Entry.decode(value)}")

    metadata_db = env.open_db(b"meta")

    print('---- Metadata ----')
    with env.begin(db=metadata_db) as txn:
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

    env.close()


if __name__ == "__main__":
    main(sys.argv)
