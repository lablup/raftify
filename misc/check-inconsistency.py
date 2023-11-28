import os
import lmdb
import rraft
from raftify.config import DEFAULT_CLUSTER_ID
from raftify.rraft_deserializer import init_rraft_py_deserializer

# TODO: Also inspect each node's metadata's inconsistency
from raftify.storage.lmdb import SNAPSHOT_KEY

# LAST_INDEX_KEY, HARD_STATE_KEY, CONF_STATE_KEY


def extract_entries(env):
    db = env.open_db(b"entries")
    with env.begin(db=db) as txn:
        cursor = txn.cursor()
        return {int(key.decode()): rraft.Entry.decode(value) for key, value in cursor}


def extract_snapshot(env) -> rraft.Snapshot:
    db = env.open_db(b"meta")
    with env.begin(db=db) as txn:
        cursor = txn.cursor()
        for key, value in cursor:
            if key == SNAPSHOT_KEY:
                return rraft.Snapshot.decode(value)


def compare_and_print_differences(node, leader_data, follower_data, data_type):
    for key, leader_value in leader_data.items():
        follower_value = follower_data.get(key)
        if follower_value != leader_value:
            print(
                f"Inconsistency in {data_type} of Node {node}: {key.decode()}, Leader: {leader_value}, Follower: {follower_value}"
            )


def main():
    init_rraft_py_deserializer()

    log_path = f"{os.getcwd()}/logs/{DEFAULT_CLUSTER_ID}"
    nodes = sorted(os.listdir(log_path))
    leader_node = nodes[0]

    leader_env = lmdb.open(f"{log_path}/{leader_node}", max_dbs=2)
    leader_entries = extract_entries(leader_env)
    # leader_snapshot = extract_snapshot(leader_env)

    for node in nodes:
        if node != leader_node:
            print("-" * 60)
            follower_env = lmdb.open(f"{log_path}/{node}", max_dbs=2)
            follower_entries = extract_entries(follower_env)
            follower_snapshot = extract_snapshot(follower_env)

            for key, leader_entry in leader_entries.items():
                if key <= follower_snapshot.get_metadata().get_index():
                    continue

                follower_entry = follower_entries.get(key)
                if follower_entry != leader_entry:
                    print(
                        f"Inconsistency found in {node} at index {key}: Leader {leader_entry}, Follower {follower_entry}"
                    )
            else:
                print(f"No inconsistencies found at {node}")

            follower_env.close()

    leader_env.close()


if __name__ == "__main__":
    main()
