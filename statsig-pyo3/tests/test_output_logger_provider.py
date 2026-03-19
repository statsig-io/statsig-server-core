import pytest
from statsig_python_core import Statsig, StatsigOptions
from statsig_python_core.statsig_python_core import StatsigUser
from mock_output_logger import MockOutputLoggerProvider


@pytest.fixture
def statsig_setup():
    log_provider = MockOutputLoggerProvider()
    log_provider.logs = []

    options = StatsigOptions()
    options.specs_url = "http://localhost"
    options.log_event_url = "http://localhost"
    options.output_logger_provider = log_provider

    yield log_provider, options


def test_output_logger_provider_with_test_param(statsig_setup):
    log_provider = MockOutputLoggerProvider(test_param="test_param")
    assert log_provider.test_param == "test_param"


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
