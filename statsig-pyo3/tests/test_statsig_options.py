from statsig_python_core import StatsigOptions
from test_observability_client import MockObservabilityClient


def test_default_statsig_options():
    options = StatsigOptions()
    assert options.specs_url is None


def test_initialize_partial_statsig_options():
    options = StatsigOptions(
        specs_url="https://cdn.statsig.com/v1",
    )
    assert options.specs_url == "https://cdn.statsig.com/v1"
    assert options.log_event_url is None


# Mostly testing that there are no errors
def test_full_statsig_options():
    options = StatsigOptions(
        specs_url="https://cdn.statsig.com/v1",
        specs_sync_interval_ms=1000,
        init_timeout_ms=1000,
        log_event_url="https://api.statsig.com/v1/log_event",
        disable_all_logging=True,
        event_logging_flush_interval_ms=1000,
        event_logging_max_queue_size=1000,
        enable_id_lists=False,
        wait_for_user_agent_init=False,
        wait_for_country_lookup_init=False,
        id_lists_url="https://cdn.statsig.com/v1/id_lists",
        id_lists_sync_interval_ms=1000,
        fallback_to_statsig_api=False,
        environment="production",
        output_log_level="debug",
        global_custom_fields={"key": "value"},
        observability_client=MockObservabilityClient(),
    )
    assert options.specs_url == "https://cdn.statsig.com/v1"
    assert options.log_event_url == "https://api.statsig.com/v1/log_event"
    assert options.specs_sync_interval_ms == 1000
    assert options.output_log_level == "debug"
