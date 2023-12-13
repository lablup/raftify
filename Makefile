build:
	cargo build --workspace

clean:
	rm -rf node-*

fmt:
	cargo fmt

open-doc:
	make build
	cargo doc --no-deps --open
