import abc
import asyncio
import importlib
import json
import os
import sys
from contextlib import suppress

import asyncclick as click

from .raft_client import RaftClient
from .raft_utils import format_all_entries, format_raft_node_debugging_info
from .utils import SocketAddr


class AbstractRaftifyCLIContext(metaclass=abc.ABCMeta):
    """
    Custom commands for cluster membership
    Implement this class to add custom commands for cluster membership
    """

    @abc.abstractmethod
    async def bootstrap_cluster(self, args, options):
        raise NotImplementedError

    @abc.abstractmethod
    async def bootstrap_follower(self, args, options):
        raise NotImplementedError

    @abc.abstractmethod
    async def add_member(self, args, options):
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


@cli.group(
    context_settings=dict(
        ignore_unknown_options=True,
    )
)
def member():
    """Cluster membership commands"""
    pass


def parse_args(args):
    options = {}
    arguments = []

    iter_args = iter(args)
    for arg in iter_args:
        if arg.startswith("--"):
            # Assume the format is --option=value
            option, value = arg[2:].split("=", 1)
            # Replace all hyphens in the option name with underscores
            option = option.replace("-", "_")
            options[option] = value
        else:
            arguments.append(arg)

    return arguments, options


def load_module_from_name(module_name):
    spec = importlib.util.find_spec(module_name)
    if spec is None:
        raise ImportError(f"Module {module_name} not found")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def load_module_from_path(module_path):
    module_name = os.path.basename(module_path).rsplit(".", 1)[0]
    module_dir = os.path.dirname(os.path.abspath(module_path))
    parent_dir, basic_dir = os.path.split(module_dir)

    sys.path.insert(0, parent_dir)

    full_package_name = f"{basic_dir}.{module_name}"
    module = importlib.import_module(full_package_name)

    sys.path.pop(0)

    return module


def load_user_implementation(module_path, module_name):
    if module_name:
        module = load_module_from_name(module_name)
    elif module_path:
        module = load_module_from_path(module_path)
    else:
        assert False, "Either module_path or module_name must be provided"

    for item_name in dir(module):
        item = getattr(module, item_name)
        if (
            isinstance(item, type)
            and issubclass(item, AbstractRaftifyCLIContext)
            and item is not AbstractRaftifyCLIContext
        ):
            return item()

    raise ImportError(
        f"No implementation of AbstractRaftifyCLIContext found in {module_path}"
    )


@cli.command(
    name="bootstrap-cluster",
    context_settings=dict(
        ignore_unknown_options=True,
    ),
)
@click.option("--module-path", "--path", "module_path", type=str, help="The path to the module.")
@click.option("--module-name", "--name", "module_name", type=str, help="The name of the module.")
@click.argument("args", nargs=-1, type=click.UNPROCESSED)
async def bootstrap_cluster(module_path, module_name, args):
    arg_list, options = parse_args(args)

    # TODO: Exclude asyncio.CancelledError exception from suppress
    with suppress(KeyboardInterrupt, asyncio.CancelledError):
        user_impl = load_user_implementation(module_path, module_name)
        await user_impl.bootstrap_cluster(arg_list, options)


@cli.command(
    name="bootstrap-follower",
    context_settings=dict(
        ignore_unknown_options=True,
    ),
)
@click.option("--module-path", "--path", "module_path", type=str, help="The path to the module.")
@click.option("--module-name", "--name", "module_name", type=str, help="The name of the module.")
@click.argument("args", nargs=-1, type=click.UNPROCESSED)
async def bootstrap_follower(module_path, module_name, args):
    arg_list, options = parse_args(args)

    # TODO: Exclude asyncio.CancelledError exception from suppress
    with suppress(KeyboardInterrupt, asyncio.CancelledError):
        user_impl = load_user_implementation(module_path, module_name)
        await user_impl.bootstrap_follower(arg_list, options)


@member.command(
    name="add",
    context_settings=dict(
        ignore_unknown_options=True,
    ),
)
@click.option("--module-path", "--path", "module_path", type=str, help="The path to the module.")
@click.option("--module-name", "--name", "module_name", type=str, help="The name of the module.")
@click.argument("args", nargs=-1, type=click.UNPROCESSED)
async def add_member(module_path, module_name, args):
    arg_list, options = parse_args(args)

    # TODO: Exclude asyncio.CancelledError exception from suppress
    with suppress(KeyboardInterrupt, asyncio.CancelledError):
        user_impl = load_user_implementation(module_path, module_name)
        await user_impl.add_member(arg_list, options)


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
