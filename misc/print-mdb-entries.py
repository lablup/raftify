import os
import sys

import lmdb
import rraft


def main(argv):
    idx = argv[1]
    assert idx.isdigit(), "idx must be a number"

    env = lmdb.open(f"{os.getcwd()}/raft-{idx}.mdb", max_dbs=2)

    entries_db = env.open_db(b"entries")
    # metadata_db = env.open_db(b"meta")

    with env.begin(db=entries_db) as txn:
        cursor = txn.cursor()
        for key, value in cursor:
            print(f"key: {int(key.decode())}, value: {rraft.Entry.decode(value)}")

    env.close()


if __name__ == "__main__":
    main(sys.argv)
