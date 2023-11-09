# Examples

The example codes can be used as test codes for raftify development and also as sample codes.

## Bootstrap and Interaction Examples

### Bootstrap cluster thorough raftify CLI

```
❯ raftify-cli bootstrap-cluster --module-path=examples/basic/cli.py --web-server=127.0.0.1:8001
```

### Bootstrap cluster thorough pure python code

```
❯ python -m examples.basic.main --bootstrap --web-server=127.0.0.1:8001
```

### Interact with RaftCluster

Here are some code examples that use RaftClient, which is available for interacting with a Raft cluster.

```
❯ python ./examples/basic/client_example.py
```
