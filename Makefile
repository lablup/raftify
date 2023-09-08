PROTO_PATH = ./raftify/protos

build-protoc:
	python -m grpc_tools.protoc --proto_path=$(PROTO_PATH) --python_out=$(PROTO_PATH) --grpc_python_out=$(PROTO_PATH) $(PROTO_PATH)/*.proto
	sed -i "" '1s/^/# type: ignore\n/' $(PROTO_PATH)/*.py
	sed -i '' 's/import raft_service_pb2 as raft__service__pb2/from . import raft_service_pb2 as raft__service__pb2/' $(PROTO_PATH)/{raft_service_pb2,raft_service_pb2_grpc}.py
	sed -i '' 's/import eraftpb_pb2 as eraftpb__pb2/from . import eraftpb_pb2 as eraftpb__pb2/' $(PROTO_PATH)/{raft_service_pb2,raft_service_pb2_grpc}.py
	protoc --proto_path=$(PROTO_PATH) --pyi_out=$(PROTO_PATH) $(PROTO_PATH)/*.proto
	python -m black $(PROTO_PATH)/*.{py,pyi}
	python -m isort $(PROTO_PATH)/*.{py,pyi}

lint:
	python -m black raftify
	python -m isort raftify
	python -m black examples
	python -m isort examples
	python -m black tests
	python -m isort tests

install:
	pip uninstall raftify -y
	pip install .

clean:
	rm -rf *.mdb

LEADER_ELECTION_TESTS = \
	test_leader_election_three_node_example \
	test_leader_election_five_node_example

test-leader-election:
	@for test in $(LEADER_ELECTION_TESTS); do \
		python -m pytest -s -v tests/leader_election.py::$$test; \
	done

DATA_REPLICATION_TESTS = \
	test_data_replication

test-data-replication:
	@for test in $(DATA_REPLICATION_TESTS); do \
		python -m pytest -s -v tests/data_replication.py::$$test; \
	done

MEMBERSHIP_CHANGE_TESTS = \
	test_membership_change

test-membership-change:
	@for test in $(MEMBERSHIP_CHANGE_TESTS); do \
		python -m pytest -s -v tests/membership_change.py::$$test; \
	done

test:
	make test-leader-election
	make test-data-replication
	make test-membership-change

reinstall:
	make clean
	make install

build-docker:
	docker build -t raftify .

run-docker:
	docker run -it raftify /bin/bash

publish:
	rm -rf dist
	python setup.py bdist_wheel
	twine upload dist/*