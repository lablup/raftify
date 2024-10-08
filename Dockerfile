FROM lukemathwalker/cargo-chef:latest-rust-1 AS chef
RUN apt-get update && apt-get install -y protobuf-compiler && apt-get install -y llvm clang libclang-dev
WORKDIR /raftify

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /raftify/recipe.json recipe.json
COPY raft-rs /raftify/raft-rs
COPY raftify-cli /raftify/raftify-cli
RUN cargo chef cook --recipe-path recipe.json
COPY . .

RUN cargo build --tests -p raftify -p harness
RUN cargo test --no-run -p raftify -p harness -v 2>&1 | grep "Executable" | awk '{print $2}' | sed "s/[\`']//g" > /raftify/test_executable_paths.txt
RUN cat /raftify/test_executable_paths.txt

FROM rust:latest AS runtime
WORKDIR /raftify
COPY --from=builder /raftify/target/debug/deps /raftify/target/debug/deps
COPY --from=builder /raftify/harness/fixtures fixtures
COPY --from=builder /raftify/harness/fixtures /fixtures

# Name of the test to be executed
ENV TEST_NAME=""
COPY --from=builder /raftify/test_executable_paths.txt /raftify/test_executable_paths.txt
CMD /bin/bash -c '\
    test_binary=$(grep "$TEST_NAME" /raftify/test_executable_paths.txt | head -1); \
    if [ -z "$test_binary" ]; then \
        echo "Error: No matching test binary found for $TEST_NAME" >&2; \
        exit 1; \
    fi; \
    echo "Executing $test_binary..."; \
    exec $test_binary'
