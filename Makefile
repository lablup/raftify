build:
	cargo build --workspace

clean:
	rm -rf node-*

install:
	cd cli && cargo install --path . && cd ..
