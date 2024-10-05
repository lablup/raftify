import argparse
import asyncio
import sys
import logging
from contextlib import suppress
from pathlib import Path

import tomli
import colorlog
from raftify import (
    Config,
    InitialRole,
    Peer,
    Peers,
    Raft,
    RaftConfig,
)


from .deserializer import register_custom_deserializer
from .web_server_api import routes, WebServer
from .state_machine import HashStore
from .utils import ensure_directory_exist, get_storage_path


def load_peers() -> Peers:
    path = Path(__file__).parent / "cluster_config.toml"
    cfg = tomli.loads(path.read_text())["raft"]["peers"]

    return Peers(
        {
            int(entry["node_id"]): Peer(
                addr=f"{entry['ip']}:{entry['port']}",
                role=InitialRole.from_str(entry["role"]),
            )
            for entry in cfg
        }
    )


def build_config(node_id: int, initial_peers: Peers) -> Config:
    raft_cfg = RaftConfig(
        id=node_id,
        election_tick=10,
        heartbeat_tick=3,
    )

    storage_path = get_storage_path("./logs", node_id)
    ensure_directory_exist(storage_path)

    cfg = Config(
        raft_cfg,
        log_dir=storage_path,
        compacted_log_dir=storage_path,
        initial_peers=initial_peers,
    )

    return cfg


class Logger:
    def __init__(self, logger) -> None:
        self.logger = logger

    def info(self, msg: str) -> None:
        self.logger.info(msg)

    def debug(self, msg: str) -> None:
        self.logger.debug(msg)

    def trace(self, msg: str) -> None:
        self.logger.debug(msg)

    def error(self, msg: str) -> None:
        self.logger.critical(msg)

    def warn(self, msg: str) -> None:
        self.logger.debug(msg)

    def fatal(self, msg: str) -> None:
        self.logger.critical(msg)


def setup_logger() -> logging.Logger:
    log_format = "%(asctime)s - " "%(log_color)s%(levelname)-8s - %(message)s%(reset)s"

    log_colors_config = {
        "DEBUG": "cyan",
        "INFO": "green",
        "WARNING": "yellow",
        "ERROR": "red",
        "CRITICAL": "red",
        "asctime": "grey",
    }

    colorlog.basicConfig(
        level=logging.DEBUG, format=log_format, log_colors=log_colors_config
    )
    return logging.getLogger()


async def main():
    if sys.platform != "win32":
        import uvloop
        uvloop.install()

    register_custom_deserializer()
    parser = argparse.ArgumentParser()
    parser.add_argument("--raft-addr", default=None)
    parser.add_argument("--web-server", default=None)
    args = parser.parse_args()

    raft_addr = args.raft_addr
    web_server_addr = args.web_server

    initial_peers = load_peers()

    node_id = initial_peers.get_node_id_by_addr(raft_addr)

    cfg = build_config(node_id, initial_peers)
    logger = Logger(setup_logger())
    store = HashStore()

    tasks = []
    raft = Raft.bootstrap(node_id, raft_addr, store, cfg, logger)
    tasks.append(raft.run())

    async with WebServer(web_server_addr, routes, {"raft": raft, "store": store}):
        await asyncio.gather(*tasks)


if __name__ == "__main__":
    with suppress(KeyboardInterrupt):
        asyncio.run(main())
