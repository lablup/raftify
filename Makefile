build:
	cargo build --workspace

clean:
	rm -rf node-*

fmt:
	cargo fmt

install-cli:
	make build
	cd cli && cargo install --path . && cd ..
