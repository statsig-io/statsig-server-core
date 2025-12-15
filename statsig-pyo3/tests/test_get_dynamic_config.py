"""
Tests focused on get_dynamic_config API.
Verifies that dynamicConfig.value is a valid JSON object (dict) and tests all related functionality.
"""
import json
import pytest
from statsig_python_core import Statsig, StatsigOptions, StatsigUser
from pytest_httpserver import HTTPServer
from utils import get_test_data_resource


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

    statsig.shutdown().wait()


def test_dynamic_config_value_is_json_dict(statsig_setup):
    statsig = statsig_setup
    config = statsig.get_dynamic_config(StatsigUser("my_user"), "big_number")

    assert isinstance(config.value, dict)
    assert config.value["foo"] == 1e21
    assert config.value["rar"] == 9999999999

    json_str = json.dumps(config.value)
    parsed = json.loads(json_str)
    assert parsed == config.value

    assert config.get_value() == config.value


def test_dynamic_config_get_methods(statsig_setup):
    statsig = statsig_setup
    config = statsig.get_dynamic_config(StatsigUser("my_user"), "big_number")

    assert config.get_float("foo", 0.0) == 1e21
    assert config.get_float("foo", 0.0) == config.value["foo"]
    assert config.get_integer("rar", 0) == 9999999999
    assert config.get_integer("rar", 0) == config.value["rar"]
    assert config.get("non_existent_key", "default") == "default"
    assert "non_existent_key" not in config.value
