from sigstat_python_core import Statsig, StatsigOptions, StatsigUser
from pytest_httpserver import HTTPServer
import json
from utils import get_test_data_resource
from werkzeug import Response, Request


def setup(httpserver: HTTPServer, logs):
    dcs_content = get_test_data_resource("eval_proj_dcs.json")
    json_data = json.loads(dcs_content)

    httpserver.expect_request(
        "/v2/download_config_specs/secret-key.json"
    ).respond_with_json(json_data)

    def _on_log_event(req: Request):
        logs.append(req.json)
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


def test_gate_exposures(httpserver: HTTPServer):
    logs = []
    statsig = setup(httpserver, logs)

    statsig.check_gate("test_public", StatsigUser("my_user"))

    statsig.flush_events().wait()
    events = get_non_diagnostic_events(logs)

    assert len(events) == 1
    assert events[0]["eventName"] == "statsig::gate_exposure"


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
