build:
	cargo build

# Test on CI
test:
	cargo test test_static_bootstrap
	cargo test test_dynamic_bootstrap
	cargo test test_data_replication
	cargo test test_leader_election_in_three_node_example

# Test on local machine with silent mode
test-silent:
	RUST_LOG=off cargo test -- --nocapture --test-threads=1
