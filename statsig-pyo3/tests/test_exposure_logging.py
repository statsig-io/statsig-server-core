from sigstat_python_core import Statsig, StatsigOptions, StatsigUser
from pytest_httpserver import HTTPServer
import json
from utils import get_test_data_resource
from werkzeug import Response, Request
import io
import gzip, zlib


def setup(httpserver: HTTPServer, logs):
    dcs_content = get_test_data_resource("eval_proj_dcs.json")
    json_data = json.loads(dcs_content)

    httpserver.expect_request(
        "/v2/download_config_specs/secret-key.json"
    ).respond_with_json(json_data)

    def _on_log_event(req: Request):
        data = req.get_data()
        json_str = gzip.decompress(data)
        req_json = json.loads(json_str)
        logs.append(req_json)
        return Response('{"success": true}')

    httpserver.expect_request("/v1/log_event").respond_with_handler(_on_log_event)

    specs_url = httpserver.url_for("/v2/download_config_specs")
    log_event_url = httpserver.url_for("/v1/log_event")

    options = StatsigOptions(specs_url, log_event_url)
    statsig = Statsig("secret-key", options)

    statsig.initialize().wait()
    return statsig


def get_non_diagnostic_events(logs):
    return [
        event
        for event in logs[0]["events"]
        if event["eventName"] != "statsig::diagnostics"
    ]


def test_shutdown_flushes(httpserver: HTTPServer):
    logs = []
    statsig = setup(httpserver, logs)

    statsig.check_gate(StatsigUser("my_user"), "test_public")

    statsig.shutdown().wait()
    events = get_non_diagnostic_events(logs)

    assert len(events) == 1
    assert events[0]["eventName"] == "statsig::gate_exposure"


def test_gate_exposures(httpserver: HTTPServer):
    logs = []
    statsig = setup(httpserver, logs)

    statsig.check_gate(StatsigUser("my_user"), "test_public")

    statsig.flush_events().wait()
    events = get_non_diagnostic_events(logs)

    assert len(events) == 1
    assert events[0]["eventName"] == "statsig::gate_exposure"


def test_layer_exposure(httpserver: HTTPServer):
    logs = []
    statsig = setup(httpserver, logs)

    layer = statsig.get_layer(StatsigUser("my_user"), "layer_with_many_params")
    statsig.flush_events().wait()
    events = get_non_diagnostic_events(logs)

    assert len(logs) == 1
    assert len(events) == 0

    layer.get_string("a_string", "ERR")
    statsig.flush_events().wait()

    logs_drop_first = logs[1:]
    events = get_non_diagnostic_events(logs_drop_first)

    assert len(logs) == 2
    assert len(events) == 1
    assert events[0]["eventName"] == "statsig::layer_exposure"


def test_custom_event(httpserver: HTTPServer):
    logs = []
    statsig = setup(httpserver, logs)

    statsig.log_event(StatsigUser("my_user"), "my_custom_event")
    statsig.flush_events().wait()

    events = get_non_diagnostic_events(logs)
    event = events[0]

    assert len(events) == 1
    assert event["eventName"] == "my_custom_event"


def test_custom_event_with_number(httpserver: HTTPServer):
    logs = []
    statsig = setup(httpserver, logs)

    statsig.log_event(StatsigUser("my_user"), "my_custom_event_with_num", 1.23)
    statsig.flush_events().wait()

    events = get_non_diagnostic_events(logs)
    event = events[0]

    assert len(events) == 1
    assert event["eventName"] == "my_custom_event_with_num"
    assert event["value"] == 1.23


def test_custom_event_with_number_and_metadata(httpserver: HTTPServer):
    logs = []
    statsig = setup(httpserver, logs)

    statsig.log_event(
        StatsigUser("my_user"), "my_custom_event_with_num", 1.23, {"some": "value"}
    )
    statsig.flush_events().wait()

    events = get_non_diagnostic_events(logs)
    event = events[0]

    assert len(events) == 1
    assert event["eventName"] == "my_custom_event_with_num"
    assert event["value"] == 1.23
    assert event["metadata"]["some"] == "value"


def test_custom_event_with_string(httpserver: HTTPServer):
    logs = []
    statsig = setup(httpserver, logs)

    statsig.log_event(StatsigUser("my_user"), "my_custom_event_with_str", "cool beans")
    statsig.flush_events().wait()

    events = get_non_diagnostic_events(logs)
    event = events[0]

    assert len(events) == 1
    assert event["eventName"] == "my_custom_event_with_str"
    assert event["value"] == "cool beans"


def test_custom_event_with_string_and_metadata(httpserver: HTTPServer):
    logs = []
    statsig = setup(httpserver, logs)

    statsig.log_event(
        StatsigUser("my_user"),
        "my_custom_event_with_str",
        "cool beans",
        {"some": "value"},
    )
    statsig.flush_events().wait()

    events = get_non_diagnostic_events(logs)
    event = events[0]

    assert len(events) == 1
    assert event["eventName"] == "my_custom_event_with_str"
    assert event["value"] == "cool beans"
    assert event["metadata"]["some"] == "value"
