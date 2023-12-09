build:
	cargo build --workspace

clean:
	rm -rf node-*

install-cli:
	cd cli && cargo install --path . && cd ..
