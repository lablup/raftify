# raftify

⚠️ This library is in a very experimental stage. The API could be broken.

raftify is a *high-level* implementation of [Raft](https://raft.github.io/), developed with the goal of making it easy and straightforward to integrate the Raft algorithm.

It uses [tikv/raft-rs](https://github.com/tikv/raft-rs) and gRPC for the network layer and [heed](https://github.com/meilisearch/heed) (LMDB wrapper) for the storage layer.

## Quick guide

I strongly recommend to read the basic [memstore example code](https://github.com/lablup/raftify/blob/main/examples/memstore/src/main.rs) to get how to use this library for starters, but here's a quick guide.

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

And similarly to `LogEntry`, you need to implement `encode` and `decode`.

- `apply`: applies a committed entry to the store.
- `snapshot`: returns snapshot data for the store.
- `restore`: applies the snapshot passed as argument.

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

let raft = Raft::build(
    node_id,
    raft_addr,
    store.clone(),
    cfg,
    logger.clone(),
    peers.clone(),
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
let join_ticket = await Raft.request_id(peer_addr);

let raft = Raft::build(
    join_ticket.reserved_id,
    raft_addr,
    store.clone(),
    cfg,
    logger.clone(),
    peers.clone(),
)?;

let raft_handle = tokio::spawn(raft.clone().run());
raft.join(request_id_resp).await;

// ...
tokio::try_join!(raft_handle)?;
```

### Manipulate FSM by RaftServiceClient

If you want to operate the FSM remotely, use `RaftServiceClient`.

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

If you want to operate FSM locally, use the RaftNode interface of the Raft object.

```rust
let mut raft_node = raft.get_raft_node();

raft_node.propose(LogEntry::Insert {
    key: 1,
    value: "test".to_string(),
}.encode().unwrap()).await;
```

It also provides a variety of other very useful APIs. Take a look at [the document]().

## Debugging

Raftify also provides a collection of CLI commands that let you check the data persisted in lmdb and the status of Raft Server.

```
$ raftify-cli debug persisted ./logs/node-1
```

```
$ raftify-cli debug node 127.0.0.1:60061
```

## Support for other language

Raftify provides bindings for the following languages.

- [Python](https://github.com/lablup/raftify/tree/main/binding/python)

## References

This library was inspired by a wide variety of previous Raft implementations.

Great thanks to all the relevant developers.

- [tikv/raft-rs](https://github.com/tikv/raft-rs) - Raft distributed consensus algorithm implemented using in this lib under the hood.
- [ritelabs/riteraft](https://github.com/ritelabs/riteraft) - A raft framework, for regular people. raftify was forked from this lib.
