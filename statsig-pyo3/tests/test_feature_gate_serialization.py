import json
import pytest
from pytest_httpserver import HTTPServer
from statsig_python_core import FeatureGate, Statsig, StatsigOptions, StatsigUser
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


def test_feature_gate_to_dict_is_json_serializable(statsig_setup):
    statsig = statsig_setup
    gate = statsig.get_feature_gate(StatsigUser("a-user"), "test_public")

    d = gate.to_dict()
    assert isinstance(d, dict)
    assert d["name"] == "test_public"
    assert isinstance(d["value"], bool)
    assert isinstance(d["details"], dict)

    json_str = json.dumps(d)
    parsed = json.loads(json_str)
    assert parsed == d


def test_feature_gate_to_py_dict_round_trip_preserves_data(statsig_setup):
    statsig = statsig_setup
    user = StatsigUser("a-user")
    gate_name = "test_public"

    raw_from_json = statsig._INTERNAL_get_feature_gate(user, gate_name)
    expected_payload = {
        "name": raw_from_json.get("name"),
        "value": raw_from_json.get("value"),
        "ruleID": raw_from_json.get("rule_id", raw_from_json.get("ruleID")),
        "idType": raw_from_json.get("id_type", raw_from_json.get("idType")),
        "details": raw_from_json.get("details"),
    }

    gate = statsig.get_feature_gate(user, gate_name)
    expected_gate = FeatureGate(gate_name, expected_payload)

    assert gate.to_dict() == expected_gate.to_dict()
