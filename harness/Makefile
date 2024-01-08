build:
	cargo build

test:
	cargo test test_static_bootstrap
	cargo test test_dynamic_bootstrap
	cargo test test_data_replication
	cargo test test_leader_election_in_three_node_example

test-silent:
	RUST_LOG=off cargo test -- --nocapture --test-threads=1
