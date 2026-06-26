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
    )
    statsig = Statsig("secret-key", options)

    statsig.initialize().wait()

    yield statsig

    # Teardown
    statsig.shutdown().wait()


def test_get_experiment_groups(statsig_setup):
    statsig = statsig_setup
    groups = statsig.get_experiment_groups("test_experiment_no_targeting")

    groups_by_name = {group["group_name"]: group["return_value"] for group in groups}

    # Only the experiment group rules are returned (the layerAssignment rule is excluded).
    assert sorted(groups_by_name.keys()) == ["Control", "Test", "Test2"]
    assert groups_by_name["Control"] == {"value": "control"}
    assert groups_by_name["Test"] == {"value": "test_1"}
    assert groups_by_name["Test2"] == {"value": "test_2"}


def test_get_experiment_groups_shape(statsig_setup):
    statsig = statsig_setup
    groups = statsig.get_experiment_groups("test_experiment_no_targeting")

    for group in groups:
        assert "group_name" in group
        assert "return_value" in group


def test_get_experiment_groups_returns_empty_for_unknown_experiment(statsig_setup):
    statsig = statsig_setup
    groups = statsig.get_experiment_groups("nonexistent_experiment")

    assert groups == []


def test_get_experiment_groups_returns_empty_for_dynamic_config(statsig_setup):
    statsig = statsig_setup
    # Dynamic configs are not experiments; should return an empty list.
    groups = statsig.get_experiment_groups("test_max_dynamic_config_size_again")

    assert groups == []


def test_get_experiment_groups_returns_empty_for_inactive_experiment(statsig_setup):
    statsig = statsig_setup
    # an_experiment1 has isActive: false; should return an empty list.
    groups = statsig.get_experiment_groups("an_experiment1")

    assert groups == []
