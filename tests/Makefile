LEADER_ELECTION_TESTS = \
	test_leader_election_three_node_example \
	test_leader_election_five_node_example

test-leader-election:
	@for test in $(LEADER_ELECTION_TESTS); do \
		python -m pytest -s -v leader_election.py::$$test; \
	done

DATA_REPLICATION_TESTS = \
	test_data_replication

test-data-replication:
	@for test in $(DATA_REPLICATION_TESTS); do \
		python -m pytest -s -v data_replication.py::$$test; \
	done

MEMBERSHIP_MANAGE_TESTS = \
	test_membership_change

test-membership-manage:
	@for test in $(MEMBERSHIP_MANAGE_TESTS); do \
		python -m pytest -s -v membership_manage.py::$$test; \
	done

test:
	make test-leader-election
	make test-data-replication
	make test-membership-manage
