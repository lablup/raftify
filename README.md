# raftify

⚠️ The library is very experimental stage.

raftify is a high-level implementation of Raft, developed with the goal of making it easy and straightforward to integrate the Raft algorithm.

It uses [tikv/raft-rs](https://github.com/tikv/raft-rs) and gRPC for the network layer and [heed](https://github.com/meilisearch/heed) (LMDB wrapper) for the storage layer.

## Quick guide

I strongly recommend to read the basic [memstore example code](https://github.com/lablup/raftify/blob/main/examples/memstore/src/main.rs) to get how to use this library for starters, but here's a quick guide.

(WIP)

## References

This library was inspired by a wide variety of previous Raft implementations.

Great thanks to all the relevant developers.

- [tikv/raft-rs](https://github.com/tikv/raft-rs) - Raft distributed consensus algorithm implemented using in this lib under the hood.
- [ritelabs/riteraft](https://github.com/ritelabs/riteraft) - A raft framework, for regular people.
- [lablup/rraft-py](https://github.com/lablup/rraft-py) - Unofficial Python Binding of the *tikv/raft-rs*.
