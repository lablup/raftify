#!/bin/bash

if [ -z "$1" ]; then
	echo "Usage: $0 <number-of-panels>"
	exit 1
fi

N=$1

PANEL_NUM=1

bootstrap() {
	tmux send-keys "RUST_LOG=debug python examples/main.py --raft-addr=127.0.0.1:60061 --web-server=127.0.0.1:8001" C-m
}

join_cluster() {
	if [ $PANEL_NUM -ne $N ]
	then
		sleep 0.5
		tmux send-keys "sleep 2; RUST_LOG=debug python examples/main.py --raft-addr=127.0.0.1:6006${PANEL_NUM} --web-server=127.0.0.1:800${PANEL_NUM} --peer-addr=127.0.0.1:60061" C-m
	fi
}

clear_terminal() {
	clear
	tmux clear-history
}

for (( i=1; i<=$N; i++ ))
do
		if (( $PANEL_NUM == 1 )); then
				clear_terminal
				bootstrap
		elif (( $PANEL_NUM % 2 == 1 )); then
				clear_terminal
				tmux select-pane -t $(($PANEL_NUM-1))
				tmux split-window -h
				join_cluster
		elif (( $PANEL_NUM % 2 == 0 )); then
				clear_terminal
				tmux select-pane -t $(($PANEL_NUM-1))
				tmux split-window -v
				join_cluster
		fi

		((PANEL_NUM=PANEL_NUM+1))
done

tmux select-pane -t $PANEL_NUM

tmux select-layout tiled
