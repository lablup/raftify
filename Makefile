build:
	cargo build --workspace

clean:
	rm -rf node-*

fmt:
	cargo fmt

open-doc:
	make build
	cargo doc --no-deps --open

test:
	cd harness && make test && cd ../

install-cli:
	cd examples/memstore && make install-cli && cd ../../
