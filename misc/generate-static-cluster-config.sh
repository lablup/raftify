#!/bin/bash
# Generate `cluster_config.toml` used in static bootstrap example.
# Used in `docker-compose.yml` of the static members example.

if [ "$#" -lt 1 ]; then
	echo "Usage: $0 addr1 addr2 addr3 ..."
	exit 1
fi

port=60061
node_id=1

for ip in "$@"
do
	echo "[[raft.peers]]"
	echo "ip = \"$ip\""
	echo "port = $port"
	echo "node_id = $node_id"
	echo "role = \"voter\""
	echo ""

	port=$((port + 1))
	node_id=$((node_id + 1))
done
