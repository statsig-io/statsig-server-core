from statsig_python_core import Statsig, StatsigOptions, StatsigUser
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


def test_override_gate_for_id(statsig_setup):
    statsig = statsig_setup
    user = StatsigUser("test-user-id")

    assert statsig.check_gate(user, "test_public")

    statsig.override_gate_for_id("test_public", "test-user-id", False)

    assert not statsig.check_gate(user, "test_public")


def test_override_dynamic_config_for_id(statsig_setup):
    statsig = statsig_setup
    user = StatsigUser("test-user-id")

    original = statsig.get_dynamic_config(user, "big_number")
    assert original.get_float("foo", 0) == 1e21

    statsig.override_dynamic_config_for_id("big_number", "test-user-id", {"foo": -1.23})

    overridden = statsig.get_dynamic_config(user, "big_number")
    assert overridden.get_float("foo", 0) == -1.23
    assert overridden.details.reason == "LocalOverride:Recognized"


def test_override_experiment_for_id(statsig_setup):
    statsig = statsig_setup
    user = StatsigUser("test-user-id")

    original = statsig.get_experiment(user, "experiment_with_many_params")
    assert original.get_string("a_string", "ERR") in ["test_1", "test_2"]  # Accept either value

    statsig.override_experiment_for_id("experiment_with_many_params", "test-user-id", {"a_string": "overridden_value"})

    overridden = statsig.get_experiment(user, "experiment_with_many_params")
    assert overridden.get_string("a_string", "ERR") == "overridden_value"
    assert overridden.details.reason == "LocalOverride:Recognized"


def test_override_experiment_by_group_name_for_id(statsig_setup):
    statsig = statsig_setup
    user = StatsigUser("test-user-id")

    original = statsig.get_experiment(user, "experiment_with_many_params")
    assert original.get_string("a_string", "ERR") in ["test_1", "test_2"]  # Accept either value

    statsig.override_experiment_by_group_name_for_id("experiment_with_many_params", "test-user-id", "Control")

    overridden = statsig.get_experiment(user, "experiment_with_many_params")
    assert overridden.get_string("a_string", "ERR") == "control"
    assert overridden.details.reason == "LocalOverride:Recognized"


def test_override_layer_for_id(statsig_setup):
    statsig = statsig_setup
    user = StatsigUser("test-user-id")

    original = statsig.get_layer(user, "layer_with_many_params")
    assert original.get_string("a_string", "ERR") in ["test_1", "test_2"]  # Accept either value

    statsig.override_layer_for_id("layer_with_many_params", "test-user-id", {"a_string": "overridden_value"})

    overridden = statsig.get_layer(user, "layer_with_many_params")
    assert overridden.get_string("a_string", "ERR") == "overridden_value"
    assert overridden.details.reason == "LocalOverride:Recognized"


def test_id_override_precedence_over_name(statsig_setup):
    statsig = statsig_setup
    user = StatsigUser("test-user-id")

    statsig.override_gate("test_public", False)
    statsig.override_gate_for_id("test_public", "test-user-id", True)

    assert statsig.check_gate(user, "test_public")


def test_override_gate_for_custom_id(statsig_setup):
    statsig = statsig_setup
    user = StatsigUser("test-user-id", custom_ids={"employee_id": "employee_id:12345"})

    assert statsig.check_gate(user, "test_public")

    statsig.override_gate_for_id("test_public", "employee_id:12345", False)

    assert not statsig.check_gate(user, "test_public")


def test_override_dynamic_config_for_custom_id(statsig_setup):
    statsig = statsig_setup
    user = StatsigUser("test-user-id", custom_ids={"employee_id": "employee_id:12345"})

    original = statsig.get_dynamic_config(user, "big_number")
    assert original.get_float("foo", 0) == 1e21

    statsig.override_dynamic_config_for_id("big_number", "employee_id:12345", {"foo": -9.87})

    overridden = statsig.get_dynamic_config(user, "big_number")
    assert overridden.get_float("foo", 0) == -9.87
    assert overridden.details.reason == "LocalOverride:Recognized"


def test_override_experiment_for_custom_id(statsig_setup):
    statsig = statsig_setup
    user = StatsigUser("test-user-id", custom_ids={"employee_id": "employee_id:12345"})

    original = statsig.get_experiment(user, "experiment_with_many_params")
    assert original.get_string("a_string", "ERR") == "test_1"  # Accept either value

    statsig.override_experiment_for_id("experiment_with_many_params", "employee_id:12345",
                                       {"a_string": "custom_id_value"})

    overridden = statsig.get_experiment(user, "experiment_with_many_params")
    assert overridden.get_string("a_string", "ERR") == "custom_id_value"
    assert overridden.details.reason == "LocalOverride:Recognized"


def test_override_experiment_by_group_name_for_custom_id(statsig_setup):
    statsig = statsig_setup
    user = StatsigUser("test-user-id", custom_ids={"employee_id": "employee_id:12345"})

    original = statsig.get_experiment(user, "experiment_with_many_params")
    assert original.get_string("a_string", "ERR") in ["test_1", "test_2"]  # Accept either value

    statsig.override_experiment_by_group_name_for_id("experiment_with_many_params", "employee_id:12345", "Control")

    overridden = statsig.get_experiment(user, "experiment_with_many_params")
    assert overridden.get_string("a_string", "ERR") == "control"
    assert overridden.details.reason == "LocalOverride:Recognized"


def test_override_layer_for_custom_id(statsig_setup):
    statsig = statsig_setup
    user = StatsigUser("test-user-id", custom_ids={"employee_id": "employee_id:12345"})

    original = statsig.get_layer(user, "layer_with_many_params")
    assert original.get_string("a_string", "ERR") in ["test_1", "test_2"]  # Accept either value

    statsig.override_layer_for_id("layer_with_many_params", "employee_id:12345", {"a_string": "custom_id_value"})

    overridden = statsig.get_layer(user, "layer_with_many_params")
    assert overridden.get_string("a_string", "ERR") == "custom_id_value"
    assert overridden.details.reason == "LocalOverride:Recognized"


def test_custom_id_override_precedence(statsig_setup):
    statsig = statsig_setup
    user = StatsigUser("test-user-id", custom_ids={"employee_id": "employee_id:12345"})

    statsig.override_gate("test_public", False)  # Name override (lowest precedence)
    statsig.override_gate_for_id("test_public", "employee_id:12345", True)  # Custom ID override
    statsig.override_gate_for_id("test_public", "test-user-id", False)  # User ID override (highest precedence)

    assert not statsig.check_gate(user, "test_public")
