# Building from Source

You can build the Raftify source code on macOS, Windows, and Linux.

Use following command to clone this repository,

```
❯ git clone --recursive https://github.com/lablup/raftify.git
```

If you already cloned it and forgot to initialize the submodules, execute the following command:

```
❯ git submodule update --init
```

# Development Environment Setup

## Test

You can run the test codes of Raftify in parallel using [nextest](https://github.com/nextest-rs/nextest).

To install `nextest`,

```
❯ cargo install cargo-nextest
```

And run the tests by the following command.

```
❯ cargo nextest run
```

> ⚠️ Note this [following issue](https://github.com/lablup/raftify/issues/165) on macOS.

## `precommit` hook setup

You can use pre-commit hooks with the following configuration.
This commit hook performs checks like `cargo fmt` and `cargo clippy` before committing.

```
❯ pip install pre-commit --break-system-packages
❯ pre-commit install
```

# Features

You can build Raftify with the following features.

By enabling or disabling the features below, you can include only the essential dependencies and source code in the build.

- `inmemory_storage`: In-memory log storage.
- `heed_storage`: [Heed](https://github.com/meilisearch/heed) log storage.
- `rocksdb_storage`: [RocksDB](https://github.com/rust-rocksdb/rust-rocksdb) log storage.
- `tls`: Enable TLS encryption for Raft server and client.
