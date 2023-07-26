#!/bin/bash
# Creates three terminal panels using tmux and runs riteraft-py on different ports in Python within each panel. 

tmux split-window -h
tmux select-pane -t 0
tmux split-window -v
tmux select-pane -t 2
tmux split-window -v

tmux select-pane -t 0
sleep 0.5
tmux send-keys 'python ./examples/memstore/main.py --bootstrap --web-server=0.0.0.0:8001' Enter

tmux select-pane -t 1
sleep 0.5
tmux send-keys 'sleep 2; and python ./examples/memstore/main.py --raft-addr=0.0.0.0:60062 --web-server=0.0.0.0:8002' Enter

tmux select-pane -t 2
sleep 0.5
tmux send-keys 'sleep 3; and python ./examples/memstore/main.py --raft-addr=0.0.0.0:60003 --web-server=0.0.0.0:8003' Enter

tmux select-pane -t 3
