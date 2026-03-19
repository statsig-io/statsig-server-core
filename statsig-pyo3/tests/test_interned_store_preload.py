from utils import get_test_data_resource_bytes, get_test_data_resource
from pytest_httpserver import HTTPServer
import pytest
import json
from statsig_python_core import InternedStore, Statsig, StatsigOptions, StatsigUser
from mock_output_logger import MockOutputLoggerProvider

EVAL_PROJ_JSON = get_test_data_resource_bytes("eval_proj_dcs.json")
DEMO_PROJ_PROTO = get_test_data_resource_bytes("demo_proj_dcs.pb.br")


@pytest.fixture
def server_setup(httpserver: HTTPServer):
    dcs_content = get_test_data_resource("eval_proj_dcs.json")
    json_data = json.loads(dcs_content)

    httpserver.expect_request(
        "/v2/download_config_specs/secret-key.json"
    ).respond_with_json(json_data)

    httpserver.expect_request("/v1/log_event").respond_with_json({"success": True})

    yield (
        httpserver.url_for("/v2/download_config_specs"),
        httpserver.url_for("/v1/log_event"),
    )


def test_interned_store_preload(server_setup):
    InternedStore.preload_multi([EVAL_PROJ_JSON, DEMO_PROJ_PROTO])

    specs_url, log_event_url = server_setup

    log_provider = MockOutputLoggerProvider()
    log_provider.logs = []

    statsig = Statsig(
        "secret-key",
        StatsigOptions(
            specs_url=specs_url,
            log_event_url=log_event_url,
            output_logger_provider=log_provider,
        ),
    )
    statsig.initialize().wait()
    gate = statsig.get_feature_gate(StatsigUser("a-user"), "test_public")
    statsig.shutdown().wait()

    assert gate.details.reason == "Network:Recognized"
    assert log_provider.error_count == 0
