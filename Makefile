build:
	cargo build --workspace

clean:
	rm -rf node-*

install-cli:
	make build
	cd cli && cargo install --path . && cd ..
