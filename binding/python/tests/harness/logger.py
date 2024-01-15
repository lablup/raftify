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
