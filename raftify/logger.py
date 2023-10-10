import abc


# TODO: maybe it could be good idea to make this class Singleton.
class AbstractRaftifyLogger(metaclass=abc.ABCMeta):
    @abc.abstractmethod
    def info(self, message: str, *args, **kwargs):
        pass

    @abc.abstractmethod
    def debug(self, message: str, *args, **kwargs):
        pass

    @abc.abstractmethod
    def warning(self, message: str, *args, **kwargs):
        pass

    @abc.abstractmethod
    def error(self, message: str, *args, **kwargs):
        pass

    @abc.abstractmethod
    def critical(self, message: str, *args, **kwargs):
        pass

    @abc.abstractmethod
    def verbose(self, message: str, *args, **kwargs):
        pass
