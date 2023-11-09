import logging
import os

import colorlog
from rraft import default_logger


def debug_mode_enabled():
    try:
        return os.environ["DEBUG"] == "1"
    except KeyError:
        return False


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

    level = logging.DEBUG if debug_mode_enabled() else logging.INFO
    colorlog.basicConfig(level=level, format=log_format, log_colors=log_colors_config)
    return logging.getLogger()


def setup_slog():
    # Set up rraft-py's slog log-level to Debug.
    # TODO: This method should be improved in rraft-py.
    os.environ["RUST_LOG"] = "DEBUG" if debug_mode_enabled() else "INFO"
    return default_logger()


slog = setup_slog()
logger = setup_logger()
