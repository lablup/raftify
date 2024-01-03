build:
	cargo build --workspace

clean:
	rm -rf node-*

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

install-cli:
	cd examples/memstore && make install-cli && cd ../../
