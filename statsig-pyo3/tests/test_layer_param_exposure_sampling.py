import pytest
from pytest_httpserver import HTTPServer
from statsig_python_core import Statsig, StatsigOptions, StatsigUser

from mock_scrapi import MockScrapi
from utils import get_test_data_resource


@pytest.fixture
def sampling_statsig_setup(httpserver: HTTPServer):
    mock_scrapi = MockScrapi(httpserver)
    dcs_content = get_test_data_resource("dcs_with_analytical_exposure_sampling.json")
    mock_scrapi.stub(
        "/v2/download_config_specs/secret-key.json",
        response=dcs_content,
        method="GET",
    )
    mock_scrapi.stub("/v1/log_event", response='{"success": true}', method="POST")

    options = StatsigOptions()
    options.specs_url = mock_scrapi.url_for_endpoint("/v2/download_config_specs")
    options.log_event_url = mock_scrapi.url_for_endpoint("/v1/log_event")
    options.output_log_level = "none"

    statsig = Statsig("secret-key", options)
    statsig.initialize().wait()
    mock_scrapi.reset()

    yield statsig, mock_scrapi

    statsig.shutdown().wait()


def test_sampled_layer_param_exposures_use_python_bridge_exposure_info(
    sampling_statsig_setup,
):
    statsig, mock_scrapi = sampling_statsig_setup

    for i in range(80):
        user = StatsigUser(f"sampled-layer-user-{i}")
        layer = statsig.get_layer(user, "json_sampled_layer")
        assert layer.get_string("param", "fallback") == "layer_value"

    statsig.flush_events().wait()

    events = mock_scrapi.get_logged_events()
    layer_events = [
        event
        for event in events
        if event.get("eventName") == "statsig::layer_exposure"
    ]

    assert len(layer_events) < 20
    assert all(
        event["metadata"]["config"] == "json_sampled_layer" for event in layer_events
    )


def test_analytical_gate_layer_param_exposures_are_not_sampled_in_python(
    sampling_statsig_setup,
):
    statsig, mock_scrapi = sampling_statsig_setup

    for i in range(40):
        user = StatsigUser(f"analytical-layer-user-{i}")
        layer = statsig.get_layer(user, "parent_layer")
        assert layer.get_string("param", "fallback") == "layer_value"

    statsig.flush_events().wait()

    events = mock_scrapi.get_logged_events()
    layer_events = [
        event
        for event in events
        if event.get("eventName") == "statsig::layer_exposure"
    ]

    assert len(layer_events) == 40
    assert all(event["metadata"]["config"] == "parent_layer" for event in layer_events)
