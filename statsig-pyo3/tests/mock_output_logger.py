from typing import List, Tuple
from statsig_python_core import OutputLoggerProvider

class MockOutputLoggerProvider(OutputLoggerProvider):
    init_called = False
    shutdown_called = False
    error_count = 0
    logs: List[Tuple[str,str, str]] = [] # (level, tag, msg)

    def __new__(cls, test_param: str = ""):
        instance = super().__new__(cls)
        instance.test_param = test_param
        return instance

    def init(self):
        self.init_called = True
    
    def debug(self, tag: str, msg: str):
        print(f"DEBUG: {tag}: {msg}")
        self.logs.append(("DEBUG", tag, msg))
    
    def info(self, tag: str, msg: str):
        print(f"INFO: {tag}: {msg}")
        self.logs.append(("INFO", tag, msg))

    def warn(self, tag: str, msg: str):
        print(f"WARN: {tag}: {msg}")
        self.logs.append(("WARN", tag, msg))

    def error(self, tag: str, msg: str):
        print(f"ERROR: {tag}: {msg}")
        self.error_count += 1
        self.logs.append(("ERROR", tag, msg))

    def shutdown(self):
        self.shutdown_called = True