# build stage
FROM rust:latest AS builder

WORKDIR /app

COPY . .
RUN apt-get update && apt-get install -y protobuf-compiler
RUN cargo clean
RUN cargo build


# program stage
FROM ubuntu:latest
# FROM rust:latest

EXPOSE 8001
# ENV PROGRAM_PATH=/app/target/debug/memstore-dynamic-members

WORKDIR /app

COPY --from=builder /app/target/debug/memstore-dynamic-members .
# CMD ["/app/target/debug/memstore-dynamic-members", "--raft-addr=127.0.0.1:60061", "--web-server=127.0.0.1:8001"]
CMD ["./memstore-dynamic-members", "--raft-addr=127.0.0.1:60061",  "--web-server=0.0.0.0:8001"]
