build:
	cargo build --workspace

clean:
	rm -rf ./logs/node-*

fmt:
	cargo fmt

lint:
	cargo clippy

open-doc:
	make build
	cargo doc --no-deps --open

unit-test:
	cd raftify && cargo test && cd ../

integration-test:
	cd harness && make test && cd ../

publish-rs:
	cargo publish -p raftify --allow-dirty --no-verify

publish-py:
	cd binding/python && make publish && cd ../../
