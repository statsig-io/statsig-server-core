import json

import pytest
from pytest_httpserver import HTTPServer
from statsig_python_core import StatsigOptions, Statsig

from utils import get_test_data_resource


@pytest.fixture
def statsig_setup(httpserver: HTTPServer):
    dcs_content = get_test_data_resource("eval_proj_dcs.json")
    json_data = json.loads(dcs_content)
    id_lists_content = get_test_data_resource("get_id_lists.json")
    id_lists_json_data = json.loads(id_lists_content)

    httpserver.expect_request(
        "/v2/download_config_specs/secret-key.json"
    ).respond_with_json(json_data)

    httpserver.expect_request("/v1/log_event").respond_with_json({"success": True})

    yield httpserver


def test_initialize_with_details_success(statsig_setup):
    mock_server = statsig_setup
    options = StatsigOptions(
        specs_url=mock_server.url_for("/v2/download_config_specs"),
        log_event_url=mock_server.url_for("/v1/log_event"),
    )
    statsig = Statsig("secret-key", options)

    init_details = statsig.initialize_with_details().result()
    assert init_details.is_config_spec_ready
    assert init_details.is_id_list_ready is None
    assert init_details.init_success
    assert init_details.duration > 0
    assert init_details.source == "Network"
    assert init_details.failure_details is None


def test_initialize_with_details_failure(statsig_setup):
    mock_server = statsig_setup
    options = StatsigOptions(
        specs_url="invalid-url",
        log_event_url=mock_server.url_for("/v1/log_event"),
    )
    statsig = Statsig("secret-key", options)

    init_details = statsig.initialize_with_details().result()
    assert not init_details.is_config_spec_ready
    assert init_details.is_id_list_ready is None
    assert init_details.init_success
    assert init_details.duration > 0
    assert init_details.source == "NoValues"
    assert init_details.failure_details is not None
