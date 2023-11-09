# Examples

The example codes can be used as test codes for raftify development and also as sample codes.

## Bootstrap cluster thorough raftify CLI

By implementing the `AbstractCLIContext` class, you can define what happens when certain commands of `raftify-cli` are executed. See `cli_context.py`

```
❯ raftify-cli bootstrap-cluster --module-path=examples/basic/cli.py --web-server=127.0.0.1:8001
```

## Bootstrap cluster thorough pure python code

You can also bootstrap the cluster directly by calling the main function. See `main.py`

```
❯ python -m examples.basic.main --bootstrap --web-server=127.0.0.1:8001
```

## Interact with cluster through `RaftClient`

Here are some code examples that use RaftClient, which is available for interacting with a Raft cluster. See `client_example.py`

```
❯ python ./examples/basic/client_example.py
```

## Interact with cluster through web server

If you can access the RaftNode object through a web server, you can directly access the message queue via the Mailbox. In this case, you need to define a list of available APIs for the web server. See `webserver.py`
