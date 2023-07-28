#!/bin/bash

N=$1

PANEL_NUM=1

bootstrap() {
	tmux send-keys "./riteraft.sh --bootstrap --web-server=0.0.0.0:8001" C-m
}

join_cluster() {
	if [ $PANEL_NUM -ne $N ]
	then
		sleep 0.5
		tmux send-keys "sleep 2; and ./riteraft.sh --raft-addr=0.0.0.0:6006${PANEL_NUM} --web-server=0.0.0.0:800${PANEL_NUM}" C-m
	fi
}

for (( i=1; i<=$N; i++ ))
do
		if (( $PANEL_NUM == 1 )); then
				bootstrap
		elif (( $PANEL_NUM % 2 == 1 )); then
				tmux select-pane -t $(($PANEL_NUM-1))
				tmux split-window -h
				join_cluster
		elif (( $PANEL_NUM % 2 == 0 )); then
				tmux select-pane -t $(($PANEL_NUM-1))
				tmux split-window -v
				join_cluster
		fi

		((PANEL_NUM=PANEL_NUM+1))
done

tmux select-pane -t $PANEL_NUM

tmux select-layout tiled