build:
	cargo build --workspace

clean:
	rm -rf node-*

fmt:
	cargo fmt

open-doc:
	make build
	cargo doc --no-deps --open

install-cli:
	cd examples/memstore && make install-cli && cd ../../
