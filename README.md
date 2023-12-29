# raftify

⚠️ The library is at an experimental stage.

raftify is a high-level implementation of TikV Raft, developed with the goal of making it easy and straightforward to integrate the Raft algorithm.

It uses gRPC for the network layer and LMDB for the storage layer.

## Quick guide

I strongly recommend to read the basic [memstore example code]() to get how to use this library for starters, but here's a quick guide.

(WIP)

## References

This library was inspired by a wide variety of previous lift implementations.

Great thanks to all the relevant developers.

- [tikv/raft-rs](https://github.com/tikv/raft-rs) - Raft distributed consensus algorithm implemented in *Rust* using in this lib under the hood.
- [ritelabs/riteraft](https://github.com/ritelabs/riteraft) - A raft framework, for regular people. Written in *Rust*.
- [lablup/rraft-py](https://github.com/lablup/rraft-py) - Unofficial Python Binding of the *tikv/raft-rs*.
