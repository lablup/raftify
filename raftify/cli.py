import abc
import asyncio
import functools
import importlib
import json
import os
import pickle
import sys
from contextlib import suppress

import asyncclick as click
from rraft import ConfChange, ConfChangeType

from .raft_client import RaftClient
from .raft_utils import format_all_entries, format_raft_node_debugging_info
from .utils import SocketAddr


class AbstractCLIContext(metaclass=abc.ABCMeta):
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

    async def remove_member(self, args, options):
        # TODO: Implement this (default)
        pass


@click.group()
def cli():
    pass


@cli.group()
def debug():
    """Group of commands debugging raft cluster"""
    pass


@cli.group(
    context_settings=dict(
        ignore_unknown_options=True,
    )
)
def member():
    """Group of commands managing cluster membership"""
    pass


def common_member_options(func):
    @click.option(
        "-p",
        "--path",
        "--module-path",
        "module_path",
        type=str,
        required=False,
        help="The path to the module.",
    )
    @click.option(
        "-m",
        "--module",
        "--module-name",
        "module_name",
        type=str,
        required=False,
        help="The name of the module.",
    )
    @functools.wraps(func)
    async def wrapper(*args, **kwargs):
        return await func(*args, **kwargs)

    return wrapper


def raw_parse_args(args):
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


def load_module_from_path(path):
    if os.path.abspath(".") not in sys.path:
        sys.path.insert(0, os.path.abspath("."))

    module_name = path.split("/")[-1].split(".")[0]
    spec = importlib.util.spec_from_file_location(module_name, path)
    if spec is None:
        raise ImportError(f"Invalid module: {path}")

    module = importlib.util.module_from_spec(spec)

    sys.modules[module_name] = module
    spec.loader.exec_module(module)

    return module


def load_user_implementation(module_path, module_name):
    if module_name:
        module = load_module_from_name(module_name)
    elif module_path:
        module = load_module_from_path(module_path)
    else:
        assert False, "Either --module-path or --module-name must be provided"

    for item_name in dir(module):
        item = getattr(module, item_name)
        if (
            isinstance(item, type)
            and issubclass(item, AbstractCLIContext)
            and item is not AbstractCLIContext
        ):
            return item()

    raise ImportError(f"No implementation of AbstractCLIContext found in {module_path}")


@cli.command(
    name="bootstrap-cluster",
    context_settings=dict(
        ignore_unknown_options=True,
    ),
)
@common_member_options
@click.argument("args", nargs=-1, type=click.UNPROCESSED)
async def bootstrap_cluster(module_path, module_name, args):
    arg_list, options = raw_parse_args(args)

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
@common_member_options
@click.argument("args", nargs=-1, type=click.UNPROCESSED)
async def bootstrap_follower(module_path, module_name, args):
    arg_list, options = raw_parse_args(args)

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
@common_member_options
@click.argument("args", nargs=-1, type=click.UNPROCESSED)
async def add_member(module_path, module_name, args):
    arg_list, options = raw_parse_args(args)

    # TODO: Exclude asyncio.CancelledError exception from suppress
    with suppress(KeyboardInterrupt, asyncio.CancelledError):
        user_impl = load_user_implementation(module_path, module_name)
        await user_impl.add_member(arg_list, options)


@member.command(name="remove")
@click.argument("node-id", type=str)
@click.argument("addr", type=str)
async def remove_member(node_id, addr):
    addr = SocketAddr.from_str(addr)

    conf_change = ConfChange.default()
    conf_change.set_node_id(int(node_id))
    conf_change.set_context(pickle.dumps([addr]))
    conf_change.set_change_type(ConfChangeType.RemoveNode)
    conf_change_v2 = conf_change.as_v2()

    client = RaftClient(addr)
    await client.change_config(conf_change_v2)


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
