import logging
import os

from rraft import default_logger


def setup_logger() -> logging.Logger:
    logging.basicConfig(level=logging.DEBUG)
    return logging.getLogger()


def setup_slog():
    # Set up rraft-py's slog log-level to Debug.
    # TODO: This method should be improved in rraft-py.
    os.environ["RUST_LOG"] = "DEBUG"
    return default_logger()


slog = setup_slog()
logger = setup_logger()
