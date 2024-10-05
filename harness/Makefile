build:
	cargo build --features "heed_storage, tls"

# Test on CI
test:
	cargo test -- --test-threads 1

# Test on local machine with silent mode
test-silent:
	RUST_LOG=off cargo test -- --nocapture --test-threads=1
