build:
	cargo build --workspace

clean:
	rm -rf ./logs/node-* && rm -rf ./harness/logs && rm -rf ./harness/.ip_counter

fmt:
	cargo fmt

lint:
	cargo clippy

open-doc:
	make build
	cargo doc --no-deps --open

test:
	cargo nextest run

publish-rs:
	cargo publish -p raftify --allow-dirty --no-verify

publish-py:
	cd binding/python && make publish && cd ../../
