import asyncclick as click
import asyncio
import json

from .raft_client import RaftClient
from .raft_utils import format_all_entries, format_raft_node_debugging_info
from .utils import SocketAddr


@click.group()
def cli():
    pass


@cli.group()
def debug():
    """Debugging commands"""
    pass


@debug.command(name='node')
@click.argument('addr', type=str)
async def debug_node(addr):
    addr = SocketAddr.from_str(addr)
    client = RaftClient(addr)
    res = await client.debug_node(timeout=5.0)
    print(format_raft_node_debugging_info(json.loads(res.result)))


@debug.command(name='entries')
@click.argument('addr', type=str)
async def debug_entries(addr):
    addr = SocketAddr.from_str(addr)
    client = RaftClient(addr)
    res = await client.debug_entries(timeout=5.0)
    print(format_all_entries(json.loads(res.result)))


@cli.command()
@click.argument('addr', type=str)
async def version(addr):
    addr = SocketAddr.from_str(addr)
    client = RaftClient(addr)
    res = await client.version(timeout=5.0)
    print(res.result)


def main():
    cli()


if __name__ == "__main__":
    asyncio.run(main())
