# build stage
FROM rust:latest AS builder

WORKDIR /app

COPY . .
RUN apt-get update && apt-get install -y protobuf-compiler
RUN cargo clean
RUN cargo build --workspace


# program stage
FROM ubuntu:latest

WORKDIR /app

COPY --from=builder /app/target/debug/memstore-dynamic-members .

# unannotate for single-node clusters
# CMD ["./memstore-dynamic-members", "--raft-addr=127.0.0.1:60061", "--web-server=0.0.0.0:8001"]
