import gzip
import json
import pytest
from statsig_python_core import Statsig, StatsigOptions, StatsigUser
from mock_scrapi import MockScrapi
from utils import get_test_data_resource
from pytest_httpserver import HTTPServer


@pytest.fixture
def statsig_setup(httpserver: HTTPServer):
    mock_scrapi = MockScrapi(httpserver)
    dcs_content = get_test_data_resource("eval_proj_dcs.json")
    mock_scrapi.stub(
        "/v2/download_config_specs/secret-key.json", response=dcs_content, method="GET"
    )
    mock_scrapi.stub("/v1/log_event", response='{"success": true}', method="POST")

    options = StatsigOptions()
    options.specs_url = mock_scrapi.url_for_endpoint("/v2/download_config_specs")
    options.log_event_url = mock_scrapi.url_for_endpoint("/v1/log_event")
    options.output_log_level = "none"

    statsig = Statsig("secret-key", options)
    statsig.initialize().wait()

    yield statsig, mock_scrapi

    # Teardown code
    statsig.shutdown().wait()


def test_shutdown_flushes(statsig_setup):
    statsig, mock_scrapi = statsig_setup

    statsig.check_gate(StatsigUser("my_user"), "test_public")
    statsig.shutdown().wait()
    events = mock_scrapi.get_logged_events()

    assert len(events) == 1
    assert events[0]["eventName"] == "statsig::gate_exposure"


def test_gate_exposures(statsig_setup):
    statsig, mock_scrapi = statsig_setup

    statsig.check_gate(StatsigUser("my_user"), "test_public")
    statsig.flush_events().wait()
    events = mock_scrapi.get_logged_events()

    assert len(events) == 1
    assert events[0]["eventName"] == "statsig::gate_exposure"


def test_layer_exposure(statsig_setup):
    statsig, mock_scrapi = statsig_setup

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


def test_custom_event(statsig_setup):
    statsig, mock_scrapi = statsig_setup

    statsig.log_event(StatsigUser("my_user"), "my_custom_event")
    statsig.flush_events().wait()

    events = mock_scrapi.get_logged_events()
    event = events[0]

    assert len(events) == 1
    assert event["eventName"] == "my_custom_event"


def test_custom_event_with_number(statsig_setup):
    statsig, mock_scrapi = statsig_setup

    statsig.log_event(StatsigUser("my_user"), "my_custom_event_with_num", 1.23)
    statsig.flush_events().wait()

    events = mock_scrapi.get_logged_events()
    event = events[0]

    assert len(events) == 1
    assert event["eventName"] == "my_custom_event_with_num"
    assert event["value"] == 1.23


def test_custom_event_with_number_and_metadata(statsig_setup):
    statsig, mock_scrapi = statsig_setup

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


def test_custom_event_with_string(statsig_setup):
    statsig, mock_scrapi = statsig_setup

    statsig.log_event(StatsigUser("my_user"), "my_custom_event_with_str", "cool beans")
    statsig.flush_events().wait()

    events = mock_scrapi.get_logged_events()
    event = events[0]

    assert len(events) == 1
    assert event["eventName"] == "my_custom_event_with_str"
    assert event["value"] == "cool beans"


def test_custom_event_with_string_and_metadata(statsig_setup):
    statsig, mock_scrapi = statsig_setup

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


def test_custom_event_with_typed_metadata(statsig_setup):
    statsig, mock_scrapi = statsig_setup

    metadata = {
        "an_int": 123,
        "a_float": 1.5,
        "a_bool": True,
        "a_none": None,
        "an_object": {"nested_int": 7, "nested_str": "x"},
        "an_array": [1, "a", False, {"k": "v"}],
    }

    statsig.log_event(
        StatsigUser("my_user"),
        "my_custom_event_with_typed_metadata",
        "value",
        metadata,
    )
    statsig.flush_events().wait()

    events = mock_scrapi.get_logged_events()
    assert len(events) == 1

    event = events[0]
    assert event["eventName"] == "my_custom_event_with_typed_metadata"
    assert event["value"] == "value"

    assert event["metadata"]["an_int"] == 123
    assert event["metadata"]["a_float"] == 1.5
    assert event["metadata"]["a_bool"] is True
    assert "a_none" not in event["metadata"] # we don't send None values
    assert event["metadata"]["an_object"] == {"nested_int": 7, "nested_str": "x"}
    assert event["metadata"]["an_array"] == [1, "a", False, {"k": "v"}]


def test_statsig_metadata(statsig_setup):
    statsig, mock_scrapi = statsig_setup

    statsig.check_gate(StatsigUser("my_user"), "test_public")
    statsig.flush_events().wait()
    request = mock_scrapi.get_requests_for_endpoint("/v1/log_event")[0]
    data = request.get_data()
    json_str = gzip.decompress(data)
    req_json = json.loads(json_str)
    statsig_metadata = req_json["statsigMetadata"]

    assert statsig_metadata["sdkType"] == "statsig-server-core-python"
    assert statsig_metadata["sdkVersion"] is not None
    assert statsig_metadata["sessionID"] is not None

    lang_version = statsig_metadata["languageVersion"]
    assert lang_version is not None and lang_version != "unknown"

    os = statsig_metadata["os"]
    assert os is not None and os != "unknown"

    arch = statsig_metadata["arch"]
    assert arch is not None and arch != "unknown"
