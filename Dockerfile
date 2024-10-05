# build stage
FROM rust:latest AS builder

WORKDIR /raftify

COPY . .
RUN apt-get update && apt-get install -y protobuf-compiler && apt-get install -y llvm clang libclang-dev
RUN cargo clean
RUN cargo build --workspace


# program stage
FROM ubuntu:latest

WORKDIR /raftify

COPY --from=builder /raftify/target/debug/memstore-dynamic-members .
COPY --from=builder /raftify/target/debug/memstore-static-members .
COPY --from=builder /raftify/examples /raftify/examples
COPY --from=builder /raftify/misc/generate-static-cluster-config.sh .
