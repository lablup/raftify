# raftify

⚠️ WARNING: This library is in a very experimental stage. The API could be broken.

raftify is a *high-level* implementation of [Raft](https://raft.github.io/), developed with the goal of making it easy and straightforward to integrate the Raft algorithm.

It uses [tikv/raft-rs](https://github.com/tikv/raft-rs) and gRPC for the network layer and [heed](https://github.com/meilisearch/heed) (LMDB wrapper) for the storage layer.

## Quick guide

I strongly recommend to read the [basic memstore example code](https://github.com/lablup/raftify/blob/main/examples/memstore/static-members/src/main.rs) to get how to use this library for starters, but here's a quick guide.

### Define your own log entry

Define the data to be stored in `LogEntry` and how to *serialize* and *deserialize* it.

```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum LogEntry {
    Insert { key: u64, value: String },
}

impl AbstractLogEntry for LogEntry {
    fn encode(&self) -> Result<Vec<u8>> {
        serialize(self).map_err(|e| e.into())
    }

    fn decode(bytes: &[u8]) -> Result<LogEntry> {
        let log_entry: LogEntry = deserialize(bytes)?;
        Ok(log_entry)
    }
}
```

### Define your application Raft FSM

Essentially, the following three methods need to be implemented for the `Store`.

- `apply`: applies a committed entry to the store.
- `snapshot`: returns snapshot data for the store.
- `restore`: applies the snapshot passed as argument.

And also similarly to `LogEntry`, you need to implement `encode` and `decode`.

```rust
#[derive(Clone, Debug)]
pub struct HashStore(pub Arc<RwLock<HashMap<u64, String>>>);

impl HashStore {
    pub fn new() -> Self {
        Self(Arc::new(RwLock::new(HashMap::new())))
    }

    pub fn get(&self, id: u64) -> Option<String> {
        self.0.read().unwrap().get(&id).cloned()
    }
}

#[async_trait]
impl AbstractStateMachine for HashStore {
    async fn apply(&mut self, data: Vec<u8>) -> Result<Vec<u8>> {
        let log_entry: LogEntry = LogEntry::decode(&data)?;
        match log_entry {
            LogEntry::Insert { ref key, ref value } => {
                let mut db = self.0.write().unwrap();
                log::info!("Inserted: ({}, {})", key, value);
                db.insert(*key, value.clone());
            }
        };
        Ok(data)
    }

    async fn snapshot(&self) -> Result<Vec<u8>> {
        Ok(serialize(&self.0.read().unwrap().clone())?)
    }

    async fn restore(&mut self, snapshot: Vec<u8>) -> Result<()> {
        let new: HashMap<u64, String> = deserialize(&snapshot[..]).unwrap();
        let mut db = self.0.write().unwrap();
        let _ = std::mem::replace(&mut *db, new);
        Ok(())
    }

    fn encode(&self) -> Result<Vec<u8>> {
        serialize(&self.0.read().unwrap().clone()).map_err(|e| e.into())
    }

    fn decode(bytes: &[u8]) -> Result<Self> {
        let db: HashMap<u64, String> = deserialize(bytes)?;
        Ok(Self(Arc::new(RwLock::new(db))))
    }
}
```

### Bootstrap a raft cluster

First bootstrap the cluster that contains the leader node.

```rust
let raft_addr = "127.0.0.1:60062".to_owned();
let node_id = 1;

let raft = Raft::bootstrap(
    node_id,
    raft_addr,
    store.clone(),
    raft_config,
    logger.clone(),
)?;

tokio::spawn(raft.clone().run());

// ...
tokio::try_join!(raft_handle)?;
```

### Join follower nodes to the cluster

Then join the follower nodes.

If peer specifies the configuration of the initial members, the cluster will operate after all member nodes are bootstrapped.

```rust
let raft_addr = "127.0.0.1:60062".to_owned();
let peer_addr = "127.0.0.1:60061".to_owned();
let join_ticket = Raft::request_id(raft_addr, peer_addr).await;

let raft = Raft::bootstrap(
    join_ticket.reserved_id,
    raft_addr,
    store.clone(),
    raft_config,
    logger.clone(),
)?;

let raft_handle = tokio::spawn(raft.clone().run());
raft.join(join_ticket).await;

// ...
tokio::try_join!(join_ticket)?;
```

### Manipulate FSM by RaftServiceClient

If you want to operate the FSM remotely, you can use [RaftServiceClient](https://docs.rs/raftify/latest/raftify/raft_service/raft_service_client/struct.RaftServiceClient.html).

```rust
let mut leader_client = create_client(&"127.0.0.1:60061").await.unwrap();

leader_client
    .propose(raft_service::ProposeArgs {
        msg: LogEntry::Insert {
            key: 1,
            value: "test".to_string(),
        }
        .encode()
        .unwrap(),
    })
    .await
    .unwrap();
```

### Manipulate FSM by RaftNode

If you want to operate FSM locally, use the [RaftNode](https://docs.rs/raftify/latest/raftify/struct.RaftNode.html) type of the [Raft](https://docs.rs/raftify/latest/raftify/struct.Raft.html) object.

```rust
let mut raft_node = raft.get_raft_node();

raft_node.propose(LogEntry::Insert {
    key: 123,
    value: "test".to_string(),
}.encode().unwrap()).await;
```

## Debugging

You can use a collection of CLI commands that let you inspect the data persisted in LMDB and the status of Raft Servers.

```
❯ raftify-cli debug persisted ./logs/node-1
---- Persisted entries ----
Key: 0, "Entry { context: [], data: [], entry_type: EntryNormal, index: 0, sync_log: false, term: 0 }"
Key: 1, "Entry { context: [], data: [], entry_type: EntryNormal, index: 1, sync_log: false, term: 1 }"
Key: 2, "Entry { context: [], data: ConfChange { change_type: AddNode, node_id: 2, context: [127.0.0.1:60062], id: 0 }, entry_type: EntryConfChange, index: 2, sync_log: false, term: 1 }"
Key: 3, "Entry { context: [], data: ConfChange { change_type: AddNode, node_id: 3, context: [127.0.0.1:60063], id: 0 }, entry_type: EntryConfChange, index: 3, sync_log: false, term: 1 }"

---- Metadata ----
HardState { term: 1, vote: 1, commit: 3 }
ConfState { voters: [1, 2, 3], learners: [], voters_outgoing: [], learners_next: [], auto_leave: false }
Snapshot { data: HashStore(RwLock { data: {}, poisoned: false, .. }), metadata: Some(SnapshotMetadata { conf_state: Some(ConfState { voters: [1, 2, 3], learners: [], voters_outgoing: [], learners_next: [], auto_leave: false }), index: 1, term: 1 }) }
Last index: 3
```

## Bootstrapping from WAL

raftify support bootstrapping cluster from WAL (Write Ahead Logs), and WAL's snapshot.

This feature is useful in cases where a failure occurs in more than the number of nodes in the quorum, requiring a restart of the cluster, or when there is a need to reboot the cluster after making a batch change to the cluster members.

Use the `restore_wal_from` and `restore_wal_snapshot_from` options in `RaftConfig`.

See [this example](https://github.com/lablup/raftify/blob/main/examples/memstore/static-members/src/main.rs) for more details.

## Support for other languages

raftify provides bindings for the following languages.

- [Python](https://github.com/lablup/raftify/tree/main/binding/python)

## References

raftify was inspired by a wide variety of previous Raft implementations.

Great thanks to all the relevant developers.

- [tikv/raft-rs](https://github.com/tikv/raft-rs) - Raft distributed consensus algorithm implemented using in this lib under the hood.
- [ritelabs/riteraft](https://github.com/ritelabs/riteraft) - A raft framework, for regular people. raftify was forked from this lib.
