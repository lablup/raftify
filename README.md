# riteraft-py

`riteraft-py` is an attempt to provide a *higher-level [Raft](https://en.wikipedia.org/wiki/Raft_(algorithm)) implementation* in Python based on *[rraft-py](https://github.com/lablup/rraft-py)*.

Please note that this repository is still experimental and has not been tested thoroughly.

Also, if you want to build featureful Python Raft implementation, `rraft-py` could be a good starting point instead of this lib.

> Note: Although this library originated from *ritelabs/riteraft*, it does not guarantee to provide the same API with *ritelabs/riteraft*.

## Why?

Since *[raft-rs](https://github.com/tikv/raft-rs)* only provides an implementation for the consensus module, some developers may face difficulties in figuring out how to use this library when they first faced with the problem.

Attempts to provide higher-level Raft implementation like *[riteraft](https://github.com/ritelabs/riteraft)* have been made to address [this issue](https://github.com/tikv/raft-rs/issues/402).

This repository starts from `riteraft` for resolving the issue in Python language runtime.

## Getting started

### Installation

#### With pip

```
$ pip install riteraft
```

### Example

In order to "raft" storage, we need to implement the Storage for it. Bellow is an example with HashStore, which is a thread-safe wrapper around an HashMap:

```py
class HashStore:
    def __init__(self):
        self._store = defaultdict(str)
        self._lock = Lock()

    def get(self, key: int) -> Optional[str]:
        with self._lock:
            return self._store.get(key)

    def apply(self, msg: bytes) -> bytes:
        with self._lock:
            message: InsertMessage = msgpack.unpackb(msg)
            self._store[message.key] = message.value
            logging.info(f"Inserted: ({message.key}, {message.value})")
            return message.value.encode()

    def snapshot(self) -> bytes:
        with self._lock:
            snapshot = copy.deepcopy(self._store)
            return msgpack.packb(snapshot)

    def restore(self, snapshot: bytes) -> None:
        with self._lock:
            new = msgpack.unpackb(snapshot)
            self._store = new
```

Only 3 methods need to be implemented for the `Store`:

* `apply`: applies a committed entry to the store.
* `snapshot`: returns snapshot data for the store.
* `restore`: applies the snapshot passed as argument.

#### Running the raft

```py
async def main() -> None:
    setup_logger()
    parser = argparse.ArgumentParser()
    parser.add_argument("--raft-addr", required=True)
    parser.add_argument("--peer-addr", default=None)
    parser.add_argument("--web-server", default=None)

    args = parser.parse_args()

    options = Options(
        raft_addr=args.raft_addr,
        peer_addr=args.peer_addr,
        web_server=args.web_server,
    )

    store = HashStore()
    raft = Raft(options.raft_addr, store, logger)
    mailbox = raft.mailbox()

    tasks = []
    if options.peer_addr:
        logger.info("Running in follower mode")
        tasks.append(raft.join(options.peer_addr))
    else:
        logger.info("Running in leader mode")
        tasks.append(raft.lead())

    runner = None
    if options.web_server:
        app = Application()
        app.add_routes(routes)
        app["state"] = {
            "store": store,
            "mailbox": mailbox,
        }

        host, port = options.web_server.split(":")
        runner = web.AppRunner(app)
        await runner.setup()
        site = web.TCPSite(runner, host, port)
        tasks.append(site.start())

    try:
        await asyncio.gather(*tasks)
    finally:
        if runner:
            await runner.cleanup()
            await runner.shutdown()


if __name__ == "__main__":
    with suppress(KeyboardInterrupt):
        asyncio.run(main())
```

The `mailbox` gives you a way to interact with the raft, for sending a message, or leaving the cluster for example.

For complete example code, please refer [this link](https://github.com/lablup/riteraft-py/blob/main/examples/memstore/main.py).

## Reference

- [ritelabs/riteraft](https://github.com/ritelabs/riteraft) - A raft framework, for regular people. Written in *Rust*.
- [lablup/rraft-py](https://github.com/lablup/rraft-py) - Unofficial Python Binding of the *tikv/raft-rs*. API using in this lib under the hood.
- [lablup/aioraft-ng](https://github.com/lablup/aioraft-ng) - Unofficial implementation of RAFT consensus algorithm written in asyncio-based *Python*.
- [tikv/raft-rs](https://github.com/tikv/raft-rs) - Raft distributed consensus algorithm implemented in *Rust*.
