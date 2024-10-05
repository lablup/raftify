FROM rust:latest AS builder

WORKDIR /raftify

COPY . .

RUN apt-get update && apt-get install -y protobuf-compiler
RUN cargo clean
RUN cargo build --workspace --features "heed_storage, tls"
