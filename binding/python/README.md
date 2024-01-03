# raftify-py

⚠️ This library is in a very experimental stage. The API could be broken.

Python binding of [*raftify*](https://github.com/lablup/raftify).

## Quick guide

I strongly recommend to read the basic [memstore example code](https://github.com/lablup/raftify/blob/main/binding/python/examples/main.py) to get how to use this library for starters, but here's a quick guide.

### Define your own log entry

Define the data to be stored in LogEntry and how to serialize and de-serialize it.

```py
class SetCommand:
    def __init__(self, key: str, value: str) -> None:
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

Essentially, the following three methods need to be implemented for the `Store`.

And similarly to `LogEntry`, you need to implement `encode` and `decode`.

- `apply`: applies a commited entry to the store.
- `snapshot`: returns snapshot data for the store.
- `restore`: applies the snapshot passed as argument.

```py
class HashStore:
    def __init__(self):
        self._store = dict()

    def get(self, key: str) -> Optional[str]:
        return self._store.get(key)

    async def apply(self, msg: bytes) -> bytes:
        message = SetCommand.decode(msg)
        self._store[message.key] = message.value
        logging.info(f'SetCommand inserted: ({message.key}, "{message.value}")')
        return msg

    async def snapshot(self) -> bytes:
        return pickle.dumps(self._store)

    async def restore(self, snapshot: bytes) -> None:
        self._store = pickle.loads(snapshot)
```

### Bootstrap a raft cluster

First bootstrap the cluster that contains the leader node.

```py
logger.info("Bootstrap new Raft Cluster")
node_id = 1
raft = Raft.build(node_id, raft_addr, store, cfg, peers)
await raft.run()
await wait_for_termination(raft)
```

### Join follower nodes to the cluster

Then join the follower nodes.

If peer specifies the configuration of the initial members, the cluster will operate after all member nodes are bootstrapped.

```py
join_ticket = await Raft.request_id(peer_addr)
node_id = join_ticket.get_reserved_id()

raft = Raft.build(node_id, raft_addr, store, cfg, peers)
await raft.run()

raft.prepare_join(join_ticket)
await raft.join()
await wait_for_termination(raft)
```

### Manipulate FSM by RaftServiceClient

If you want to operate the FSM remotely, use the `RaftServiceClient`.

```py
client = await RaftServiceClient.build("127.0.0.1:60061")
client.prepare_propose(SetCommand("1", "A").encode())
await client.propose()
```

### Manipulate FSM by RaftNode

If you want to operate FSM locally, use the RaftNode interface of the Raft object

```py
raft_node = raft.get_raft_node()
raft_node.prepare_proposal(message.encode())
await raft_node.propose()
```

### Debugging

Raftify also provides a collection of CLI commands that let you check the data persisted in lmdb and the status of Raft Server.

```
$ raftify_cli debug persisted ./logs/node-1
```

```
$ raftify_cli debug node 127.0.0.1:60061
```
