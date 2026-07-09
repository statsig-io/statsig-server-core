from statsig_python_core import SpecAdapterConfig, StatsigOptions
from test_observability_client import MockObservabilityClient


def test_default_statsig_options():
    options = StatsigOptions()
    assert options.specs_url is None
    assert options.service_name is None


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
        service_name="statsig-python-service",
        output_log_level="debug",
        global_custom_fields={"key": "value"},
        observability_client=MockObservabilityClient(),
    )
    assert options.specs_url == "https://cdn.statsig.com/v1"
    assert options.log_event_url == "https://api.statsig.com/v1/log_event"
    assert options.specs_sync_interval_ms == 1000
    assert options.output_log_level == "debug"
    assert options.service_name == "statsig-python-service"


def test_exposure_dedupe_max_keys_defaults_to_none():
    options = StatsigOptions()
    assert options.exposure_dedupe_max_keys is None


def test_exposure_dedupe_max_keys_round_trips():
    # Settable via constructor and as an attribute; the getter reads the same
    # field forwarded to the core StatsigOptions.
    options = StatsigOptions(exposure_dedupe_max_keys=250_000)
    assert options.exposure_dedupe_max_keys == 250_000

    options.exposure_dedupe_max_keys = 5
    assert options.exposure_dedupe_max_keys == 5


def test_exposure_dedupe_max_keys_appended_without_breaking_positional_args():
    # exposure_dedupe_max_keys is appended AFTER the pre-existing constructor
    # params, so existing positional calls keep binding to the same options and
    # the new field defaults to None. The 11th positional arg is enable_id_lists;
    # if the new param had been inserted mid-list, `True` would bind to it here.
    options = StatsigOptions(
        "https://cdn.statsig.com/v1",  # specs_url
        None,  # specs_sync_interval_ms
        None,  # init_timeout_ms
        None,  # log_event_url
        None,  # disable_all_logging
        None,  # disable_network
        None,  # log_event_connection_reuse
        None,  # event_logging_flush_interval_ms
        None,  # event_logging_max_queue_size
        None,  # event_logging_max_pending_batch_queue_size
        True,  # enable_id_lists
    )
    assert options.specs_url == "https://cdn.statsig.com/v1"
    assert options.enable_id_lists is True
    assert options.exposure_dedupe_max_keys is None


def _assert_no_tls(config):
    assert config.adapter_type == "network_grpc_websocket"
    assert config.specs_url == "grpc://localhost:50051"
    assert config.init_timeout_ms == 3000
    assert config.authentication_mode is None
    assert config.ca_cert_path is None
    assert config.client_cert_path is None
    assert config.client_key_path is None
    assert config.domain_name is None


def test_spec_adapter_config_backward_compatible():
    # The TLS params are appended after the original positional parameters, so
    # both the pre-TLS positional call and the keyword call must still bind the
    # original three args and default the new fields to None.
    positional = SpecAdapterConfig("network_grpc_websocket", "grpc://localhost:50051", 3000)
    keyword = SpecAdapterConfig(
        adapter_type="network_grpc_websocket",
        specs_url="grpc://localhost:50051",
        init_timeout_ms=3000,
    )
    _assert_no_tls(positional)
    _assert_no_tls(keyword)


def test_spec_adapter_config_tls_fields_round_trip():
    config = SpecAdapterConfig(
        adapter_type="network_grpc_websocket",
        specs_url="grpc://localhost:50051",
        authentication_mode="mtls",
        ca_cert_path="/certs/ca.pem",
        client_cert_path="/certs/client.pem",
        client_key_path="/certs/client.key",
        domain_name="statsig.local",
    )
    assert config.adapter_type == "network_grpc_websocket"
    assert config.specs_url == "grpc://localhost:50051"
    assert config.authentication_mode == "mtls"
    assert config.ca_cert_path == "/certs/ca.pem"
    assert config.client_cert_path == "/certs/client.pem"
    assert config.client_key_path == "/certs/client.key"
    assert config.domain_name == "statsig.local"
