import pytest
from statsig_python_core import Statsig, StatsigUser


def test_error_boundary_handles_wrong_types():
    statsig = Statsig("secret-test-key")

    result = statsig.check_gate(123, "gate_name")  # Invalid user type
    assert result is None

    user = StatsigUser("user_123")
    result = statsig.check_gate(user, 123)  # Invalid gate name type
    assert result is None

    result = statsig.get_dynamic_config(123, "config_name")
    assert result is None

    result = statsig.get_experiment(user, 123)
    assert result is None

    statsig.log_event(123, "event_name")  # Should not raise
    statsig.log_event(user, 123)  # Should not raise
