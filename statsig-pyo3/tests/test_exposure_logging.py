from sigstat_python_core import Statsig, StatsigOptions, StatsigUser

from mock_scrapi import MockScrapi
from utils import get_test_data_resource
from pytest_httpserver import HTTPServer


def setup_better(httpserver: HTTPServer):
    mock_scrapi = MockScrapi(httpserver)
    dcs_content = get_test_data_resource("eval_proj_dcs.json")
    mock_scrapi.stub("/v2/download_config_specs/secret-key.json", response=dcs_content, method="GET")
    mock_scrapi.stub("/v1/log_event", response='{"success": true}', method="POST")

    options = StatsigOptions()
    options.specs_url = mock_scrapi.url_for_endpoint("/v2/download_config_specs")
    options.log_event_url = mock_scrapi.url_for_endpoint("/v1/log_event")

    statsig = Statsig("secret-key", options)
    statsig.initialize().wait()
    return statsig, mock_scrapi


def test_shutdown_flushes(httpserver: HTTPServer):
    statsig, mock_scrapi = setup_better(httpserver)

    statsig.check_gate(StatsigUser("my_user"), "test_public")

    statsig.shutdown().wait()
    events = mock_scrapi.get_logged_events()

    assert len(events) == 1
    assert events[0]["eventName"] == "statsig::gate_exposure"


def test_gate_exposures(httpserver: HTTPServer):
    statsig, mock_scrapi = setup_better(httpserver)

    statsig.check_gate(StatsigUser("my_user"), "test_public")

    statsig.flush_events().wait()
    events = mock_scrapi.get_logged_events()

    assert len(events) == 1
    assert events[0]["eventName"] == "statsig::gate_exposure"


def test_layer_exposure(httpserver: HTTPServer):
    statsig, mock_scrapi = setup_better(httpserver)

    layer = statsig.get_layer(StatsigUser("my_user"), "layer_with_many_params")
    statsig.flush_events().wait()

    log_requests = mock_scrapi.get_requests_for_endpoint("/v1/log_event")
    events = mock_scrapi.get_logged_events()

    assert len(log_requests) == 1
    assert len(events) == 0

    layer.get_string("a_string", "ERR")
    statsig.flush_events().wait()

    log_requests = mock_scrapi.get_requests_for_endpoint("/v1/log_event")
    events = mock_scrapi.get_logged_events()

    assert len(log_requests) == 2
    assert len(events) == 1
    assert events[0]["eventName"] == "statsig::layer_exposure"


def test_custom_event(httpserver: HTTPServer):
    statsig, mock_scrapi = setup_better(httpserver)

    statsig.log_event(StatsigUser("my_user"), "my_custom_event")
    statsig.flush_events().wait()

    events = mock_scrapi.get_logged_events()
    event = events[0]

    assert len(events) == 1
    assert event["eventName"] == "my_custom_event"


def test_custom_event_with_number(httpserver: HTTPServer):
    statsig, mock_scrapi = setup_better(httpserver)

    statsig.log_event(StatsigUser("my_user"), "my_custom_event_with_num", 1.23)
    statsig.flush_events().wait()

    events = mock_scrapi.get_logged_events()
    event = events[0]

    assert len(events) == 1
    assert event["eventName"] == "my_custom_event_with_num"
    assert event["value"] == 1.23


def test_custom_event_with_number_and_metadata(httpserver: HTTPServer):
    statsig, mock_scrapi = setup_better(httpserver)

    statsig.log_event(
        StatsigUser("my_user"), "my_custom_event_with_num", 1.23, {"some": "value"}
    )
    statsig.flush_events().wait()

    events = mock_scrapi.get_logged_events()
    event = events[0]

    assert len(events) == 1
    assert event["eventName"] == "my_custom_event_with_num"
    assert event["value"] == 1.23
    assert event["metadata"]["some"] == "value"


def test_custom_event_with_string(httpserver: HTTPServer):
    statsig, mock_scrapi = setup_better(httpserver)

    statsig.log_event(StatsigUser("my_user"), "my_custom_event_with_str", "cool beans")
    statsig.flush_events().wait()

    events = mock_scrapi.get_logged_events()
    event = events[0]

    assert len(events) == 1
    assert event["eventName"] == "my_custom_event_with_str"
    assert event["value"] == "cool beans"


def test_custom_event_with_string_and_metadata(httpserver: HTTPServer):
    statsig, mock_scrapi = setup_better(httpserver)

    statsig.log_event(
        StatsigUser("my_user"),
        "my_custom_event_with_str",
        "cool beans",
        {"some": "value"},
    )
    statsig.flush_events().wait()

    events = mock_scrapi.get_logged_events()
    event = events[0]

    assert len(events) == 1
    assert event["eventName"] == "my_custom_event_with_str"
    assert event["value"] == "cool beans"
    assert event["metadata"]["some"] == "value"
