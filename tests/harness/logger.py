import logging
import os

from rraft import default_logger

from raftify import AbstractRaftifyLogger


def setup_logger() -> logging.Logger:
    logging.basicConfig(level=logging.DEBUG)
    return logging.getLogger()


def setup_slog():
    # Set up rraft-py's slog log-level to Debug.
    # TODO: This method should be improved in rraft-py.
    os.environ["RUST_LOG"] = "DEBUG"
    return default_logger()


class RaftifyLogger(AbstractRaftifyLogger):
    def __init__(self):
        self._log = logging.getLogger("ai.backend.manager.scheduler")

    def info(self, message: str, *args, **kwargs):
        self._log.info(message, *args, **kwargs)

    def debug(self, message: str, *args, **kwargs):
        self._log.debug(message, *args, **kwargs)

    def warning(self, message: str, *args, **kwargs):
        self._log.warning(message, *args, **kwargs)

    def error(self, message: str, *args, **kwargs):
        self._log.error(message, *args, **kwargs)


slog = setup_slog()
logger = setup_logger()
