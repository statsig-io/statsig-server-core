from statsig_python_core import OutputLoggerProviderBase

class OutputLoggerProvider(OutputLoggerProviderBase):
    def __new__(cls, *args, **kwargs):
        return super().__new__(cls)

    def __init__(self, *args, **kwargs):
        super().__init__()
        self.init_fn = self.init
        self.debug_fn = self.debug
        self.info_fn = self.info
        self.warn_fn = self.warn
        self.error_fn = self.error
        self.shutdown_fn = self.shutdown

    def init(self):
        pass

    def debug(self, tag: str, msg: str):
        pass

    def info(self, tag: str, msg: str):
        pass

    def warn(self, tag: str, msg: str):
        pass

    def error(self, tag: str, msg: str):
        pass

    def shutdown(self):
        pass


