LEADER_ELECTION_TESTS = \
	test_leader_election_three_node_example \
	test_leader_election_five_node_example

test-leader-election:
	@for test in $(LEADER_ELECTION_TESTS); do \
		python3 -m pytest -s -v leader_election.py::$$test; \
	done

DATA_REPLICATION_TESTS = \
	test_data_replication

test-data-replication:
	@for test in $(DATA_REPLICATION_TESTS); do \
		python3 -m pytest -s -v data_replication.py::$$test; \
	done

test-membership-manage:
	@for test in $(MEMBERSHIP_MANAGE_TESTS); do \
		python3 -m pytest -s -v membership_manage.py::$$test; \
	done

NETWORK_TESTS = \
	test_message_rerouting

test-network:
	@for test in $(NETWORK_TESTS); do \
		python3 -m pytest -s -v network.py::$$test; \
	done

test:
	make test-leader-election
	make test-data-replication
	make test-network
