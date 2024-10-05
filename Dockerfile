# build stage
FROM rust:latest AS builder

WORKDIR /raftify

COPY . .
RUN apt-get update && apt-get install -y protobuf-compiler
RUN cargo clean
RUN cargo build --workspace


# program stage
FROM ubuntu:latest

WORKDIR /raftify

COPY --from=builder /raftify/target/debug/memstore-dynamic-members .
COPY --from=builder /raftify/target/debug/memstore-static-members .
COPY --from=builder /raftify/examples .
COPY --from=builder /misc/generate-static-cluster-config.sh .