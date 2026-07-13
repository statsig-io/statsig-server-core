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
    result = statsig.get_experiment_groups("test_experiment_no_targeting")

    assert result["is_experiment_active"] is True

    groups_by_name = {group["group_name"]: group for group in result["groups"]}

    # Only the experiment group rules are returned (the layerAssignment rule is excluded).
    assert sorted(groups_by_name.keys()) == ["Control", "Test", "Test2"]
    assert groups_by_name["Control"]["return_value"] == {"value": "control"}
    assert groups_by_name["Control"]["rule_id"] == "54QJztEPRLXK7ZCvXeY9q4"
    assert groups_by_name["Control"]["id_type"] == "userID"
    assert groups_by_name["Test"]["return_value"] == {"value": "test_1"}
    assert groups_by_name["Test2"]["return_value"] == {"value": "test_2"}


def test_get_experiment_groups_shape(statsig_setup):
    statsig = statsig_setup
    result = statsig.get_experiment_groups("test_experiment_no_targeting")

    assert "is_experiment_active" in result
    for group in result["groups"]:
        assert "group_name" in group
        assert "rule_id" in group
        assert "id_type" in group
        assert "return_value" in group
        assert group["id_type"] == "userID"


def test_get_experiment_groups_returns_none_for_unknown_experiment(statsig_setup):
    statsig = statsig_setup
    result = statsig.get_experiment_groups("nonexistent_experiment")

    assert result["is_experiment_active"] is None
    assert result["groups"] == []


def test_get_experiment_groups_returns_none_for_dynamic_config(statsig_setup):
    statsig = statsig_setup
    # Dynamic configs are not experiments; is_experiment_active should be None.
    result = statsig.get_experiment_groups("test_max_dynamic_config_size_again")

    assert result["is_experiment_active"] is None
    assert result["groups"] == []


def test_get_experiment_groups_returns_groups_for_inactive_experiment(statsig_setup):
    statsig = statsig_setup
    # test_switchback has isActive: false; groups are still returned along with the flag.
    result = statsig.get_experiment_groups("test_switchback")

    assert result["is_experiment_active"] is False

    # Only the experiment group rules are returned (non-group rules are excluded).
    group_names = sorted(group["group_name"] for group in result["groups"])
    assert group_names == ["Control", "Test"]
