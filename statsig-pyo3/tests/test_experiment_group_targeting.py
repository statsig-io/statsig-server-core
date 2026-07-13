from statsig_python_core import Statsig, StatsigOptions
from pytest_httpserver import HTTPServer
import json
from utils import get_test_data_resource
import pytest


@pytest.fixture
def statsig_setup(httpserver: HTTPServer):
    dcs_content = get_test_data_resource("eval_proj_dcs.json")
    json_data = json.loads(dcs_content)

    httpserver.expect_request(
        "/v2/download_config_specs/secret-key.json"
    ).respond_with_json(json_data)

    httpserver.expect_request("/v1/log_event").respond_with_json({"success": True})

    options = StatsigOptions(
        specs_url=httpserver.url_for("/v2/download_config_specs"),
        log_event_url=httpserver.url_for("/v1/log_event"),
        output_log_level="debug",
    )
    statsig = Statsig("secret-key", options)

    statsig.initialize().wait()

    yield statsig

    statsig.shutdown().wait()


def test_get_experiment_by_group_name(statsig_setup):
    statsig = statsig_setup

    control = statsig.get_experiment_by_group_name(
        "test_experiment_no_targeting", "Control"
    )
    assert control.group_name == "Control"
    assert control.rule_id == "54QJztEPRLXK7ZCvXeY9q4"
    assert control.id_type == "userID"
    assert control.get_string("value", "ERR") == "control"

    test = statsig.get_experiment_by_group_name(
        "test_experiment_no_targeting", "Test"
    )
    assert test.group_name == "Test"
    assert test.get_string("value", "ERR") == "test_1"


def test_get_experiment_by_group_name_unrecognized(statsig_setup):
    statsig = statsig_setup

    experiment = statsig.get_experiment_by_group_name(
        "not_an_experiment", "Control"
    )
    assert experiment.group_name is None
    assert experiment.rule_id == ""


def test_get_experiment_by_group_id_advanced(statsig_setup):
    statsig = statsig_setup

    experiment = statsig.get_experiment_by_group_id_advanced(
        "test_experiment_no_targeting", "54QJztEPRLXK7ZCvXeY9q4"
    )
    assert experiment.group_name == "Control"
    assert experiment.rule_id == "54QJztEPRLXK7ZCvXeY9q4"
    assert experiment.id_type == "userID"
    assert experiment.get_string("value", "ERR") == "control"


def test_get_experiment_by_group_id_advanced_unrecognized(statsig_setup):
    statsig = statsig_setup

    experiment = statsig.get_experiment_by_group_id_advanced(
        "test_experiment_no_targeting", "not_a_group_id"
    )
    assert experiment.group_name is None
    assert experiment.rule_id == ""


def test_get_experiment_by_group_id_advanced_unknown_experiment(statsig_setup):
    statsig = statsig_setup

    experiment = statsig.get_experiment_by_group_id_advanced(
        "not_an_experiment", "54QJztEPRLXK7ZCvXeY9q4"
    )
    assert experiment.group_name is None
    assert experiment.rule_id == ""
