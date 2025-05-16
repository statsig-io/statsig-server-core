import json
from typing import List, Tuple
import pytest
from pytest_httpserver import HTTPServer
from statsig_python_core import OutputLoggerProvider, Statsig, StatsigOptions
from statsig_python_core.statsig_python_core import StatsigUser

from utils import get_test_data_resource


class MockOutputLoggerProvider(OutputLoggerProvider):
    init_called = False
    shutdown_called = False
    logs: List[Tuple[str,str, str]] = [] # (level, tag, msg)

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
        self.logs.append(("ERROR", tag, msg))

    def shutdown(self):
        self.shutdown_called = True

@pytest.fixture
def statsig_setup():
    log_provider = MockOutputLoggerProvider()
    log_provider.logs = []

    options = StatsigOptions()
    options.specs_url = "http://localhost"
    options.log_event_url = "http://localhost"
    options.output_logger_provider = log_provider

    yield log_provider, options

def test_output_logger_provider(statsig_setup):
    log_provider, options = statsig_setup

    options.output_log_level = "debug"
    statsig = Statsig("secret-key", options)

    statsig.initialize().wait()

    assert log_provider.init_called
    assert len(log_provider.logs) > 0

    statsig.shutdown().wait()

def test_output_logger_filter_level(statsig_setup):
    log_provider, options = statsig_setup
    options.output_log_level = "warn"
    statsig = Statsig("secret-key", options)

    statsig.initialize().wait()
    statsig.check_gate(StatsigUser("a-user"), "test_public")
    statsig.shutdown().wait()

    assert log_provider.init_called
    assert log_provider.shutdown_called

    debug_logs = next((log for log in log_provider.logs if log[0] == "DEBUG"), None)
    assert debug_logs is None

    warn_logs = next((log for log in log_provider.logs if log[0] == "WARN"), None)
    assert warn_logs is not None

    assert debug_logs is None
    assert warn_logs is not None

    