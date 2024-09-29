# Building from Source

You can use this command to clone the repository:

```
❯ git clone --recursive https://github.com/lablup/raftify.git
```

If you already cloned it and forgot to initialize the submodules, execute the following command:

```
❯ git submodule update --init
```

# Development environment setup

## precommit hook setup

You can use pre-commit hooks with the following configuration.
This commit hook performs checks like cargo fmt and cargo clippy before committing.

```
❯ pip install pre-commit --break-system-packages
❯ pre-commit install
```
