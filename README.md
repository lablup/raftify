# raftify

`raftify` is a high-level implementation of the Raft algorithm, developed with the aim to easily and simply integrate the Raft algorithm into any Python application.

It uses [rraft-py](https://github.com/lablup/rraft-py) under the hood, a binding of [tikv/raft-rs](https://github.com/tikv/raft-rs), a well-tested implementation of Raft.

And `raftify` is also further refined fork of [riteraft](https://github.com/ritelabs/riteraft).

It uses gRPC for the network layer and LMDB for the storage layer. If you prefer a lower-level Raft implementation instead of these abstractions, you can consider to using [rraft-py](https://github.com/lablup/rraft-py).

## Features

⚠️ `raftify` is still an very experimental library and development is in progress. Currently, the following features are supported.

- [x] Leader election
- [x] Log replication
- [ ] Log compaction
- [x] Membership changes
- [x] Message rerouting to the leader node
- [ ] Writing to leader's disk in parallel
- [ ] Automatic stepping down when the leader loses quorum
- [ ] Leadership transfer extension
- [ ] Prevote protocol

## Getting started

You can create a containerized dev environment for testing raftify and launch a Raft cluster right away.

```
$ docker build -t raftify .
$ docker run -it raftify /bin/bash
```

In the container, enter the following commands for creating a cluster with three nodes.

```
$ tmux
$ ./misc/bootstrap-memstore-cluster.tmux.sh 4
```

And then you can use the `curl` command to test the cluster. For example, you can use the following command to put a key-value pair into the cluster.

```
$ curl http://localhost:8001/put/1/A
```

## Quick guide

I strongly recommend to read the [memstore example code](https://github.com/lablup/raftify/blob/main/examples/memstore/main.py) to get how to use this library for starters, but here's a quick guide.

### Define your own log entry

```py
class SetCommand:
    def __init__(self, key: int, value: str) -> None:
        self.key = key
        self.value = value

    def encode(self) -> bytes:
        return pickle.dumps(self.__dict__)

    @classmethod
    def decode(cls, packed: bytes) -> "SetCommand":
        unpacked = pickle.loads(packed)
        return cls(unpacked["key"], unpacked["value"])
```

### Define your application Raft FSM

```py
class HashStore(FSM):
    def __init__(self):
        self._store = dict()
        self._lock = Lock()

    def get(self, key: int) -> Optional[str]:
        with self._lock:
            return self._store.get(key)

    async def apply(self, msg: bytes) -> bytes:
        with self._lock:
            message = SetCommand.decode(msg)
            self._store[message.key] = message.value
            logging.info(f'SetCommand inserted: ({message.key}, "{message.value}")')
            return pickle.dumps(message.value)

    async def snapshot(self) -> bytes:
        with self._lock:
            return pickle.dumps(self._store)

    async def restore(self, snapshot: bytes) -> None:
        with self._lock:
            self._store = pickle.loads(snapshot)
```

### Bootstrap a raft cluster

```py
cluster = RaftCluster(raft_addr, store, logger)
logger.info("Bootstrap a raft cluster")
await cluster.bootstrap_cluster()
```

### Join follower nodes to the cluster

```py
logger.info("Running follower node")
cluster = RaftCluster(raft_addr, store, logger)
await cluster.join_cluster(raft_addr, peer_addrs, follower_role)
```

<!-- ## Installation

```
$ pip install raftify
``` -->

## References

This library was inspired by a wide variety of previous lift implementations.

Great thanks to all the relevant developers.

- [lablup/rraft-py](https://github.com/lablup/rraft-py) - Unofficial Python Binding of the *tikv/raft-rs*. API using in this lib under the hood.
- [tikv/raft-rs](https://github.com/tikv/raft-rs) - Raft distributed consensus algorithm implemented in *Rust*.
- [ritelabs/riteraft](https://github.com/ritelabs/riteraft) - A raft framework, for regular people. Written in *Rust*.
- [lablup/riteraft-py](https://github.com/lablup/riteraft-py) - Porting version of *riteraft*
- [lablup/aioraft-ng](https://github.com/lablup/aioraft-ng) - Unofficial implementation of RAFT consensus algorithm written in asyncio-based *Python*.
- [canonical/raft](https://github.com/canonical/raft) - *C* implementation of the Raft consensus protocol
- [MarinPostma/raft-frp](https://github.com/MarinPostma/raft-frp) - raft, for regular people
