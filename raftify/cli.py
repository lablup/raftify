import abc
import asyncio
from contextlib import suppress
import importlib
import json

import asyncclick as click

from .raft_client import RaftClient
from .raft_utils import format_all_entries, format_raft_node_debugging_info
from .utils import SocketAddr


class RaftifyContext(metaclass=abc.ABCMeta):
    """
    Custom commands for cluster membership
    Implement this class to add custom commands for cluster membership
    """

    @abc.abstractmethod
    async def bootstrap_cluster(self, args):
        raise NotImplementedError

    @abc.abstractmethod
    async def bootstrap_member(self, args):
        raise NotImplementedError

    @abc.abstractmethod
    async def add_member(self, args):
        raise NotImplementedError

    async def remove_member(self):
        # TODO: Implement this (default)
        pass


@click.group()
def cli():
    pass


@cli.group()
def debug():
    """Debugging commands"""
    pass


@cli.group()
def member():
    """Cluster membership commands"""
    pass


def load_module_from_path(path):
    spec = importlib.util.spec_from_file_location("module.name", path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def load_user_implementation(module_path):
    module = load_module_from_path(module_path)
    for item_name in dir(module):
        item = getattr(module, item_name)
        if (
            isinstance(item, type)
            and issubclass(item, RaftifyContext)
            and item is not RaftifyContext
        ):
            return item()
    raise ImportError(f"No implementation of RaftifyContext found in {module_path}")


@cli.command(name="bootstrap-cluster")
@click.argument("module_pth", type=str)
@click.argument("args")
async def bootstrap_cluster(module_pth, args):
    # TODO: Exclude asyncio.CancelledError exception from suppress
    with suppress(KeyboardInterrupt, asyncio.CancelledError):
        user_impl = load_user_implementation(module_pth)
        await user_impl.bootstrap_cluster(args)


@member.command(name="add")
@click.argument("module_pth", type=str)
@click.argument("arg1")
@click.argument("arg2")
async def add_member(module_pth, arg1, arg2):
    # TODO: Exclude asyncio.CancelledError exception from suppress
    with suppress(KeyboardInterrupt, asyncio.CancelledError):
        user_impl = load_user_implementation(module_pth)
        await user_impl.add_member(arg1, arg2)


@member.command(name="remove")
@click.argument("module_pth", type=str)
@click.argument("args")
async def remove_member():
    # TODO: Implement this (default)
    pass


@debug.command(name="node")
@click.argument("addr", type=str)
async def debug_node(addr):
    addr = SocketAddr.from_str(addr)
    client = RaftClient(addr)
    res = await client.debug_node(timeout=5.0)
    print(format_raft_node_debugging_info(json.loads(res.result)))


@debug.command(name="entries")
@click.argument("addr", type=str)
async def debug_entries(addr):
    addr = SocketAddr.from_str(addr)
    client = RaftClient(addr)
    res = await client.debug_entries(timeout=5.0)
    print(format_all_entries(json.loads(res.result)))


@cli.command()
@click.argument("addr", type=str)
async def version(addr):
    addr = SocketAddr.from_str(addr)
    client = RaftClient(addr)
    res = await client.version(timeout=5.0)
    print(res.result)


def main():
    cli()


if __name__ == "__main__":
    asyncio.run(main())
