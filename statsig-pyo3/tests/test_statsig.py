from sigstat_python_core import Statsig, StatsigOptions, StatsigUser
from pytest_httpserver import HTTPServer
import json
from utils import get_test_data_resource


def setup(httpserver: HTTPServer):
    dcs_content = get_test_data_resource("eval_proj_dcs.json")
    json_data = json.loads(dcs_content)

    httpserver.expect_request(
        "/v2/download_config_specs/secret-key.json"
    ).respond_with_json(json_data)

    specs_url = httpserver.url_for("/v2/download_config_specs")

    options = StatsigOptions(specs_url)
    statsig = Statsig("secret-key", options)

    statsig.initialize().wait()
    return statsig


def test_check_gate(httpserver: HTTPServer):
    statsig = setup(httpserver)

    assert statsig.check_gate(StatsigUser("a-user"), "test_public")


def test_get_feature_gate(httpserver: HTTPServer):
    statsig = setup(httpserver)
    gate = statsig.get_feature_gate(StatsigUser("a-user"), "test_public")

    assert gate.value
    assert gate.name == "test_public"
    assert gate.rule_id == "6X3qJgyfwA81IJ2dxI7lYp"
    assert gate.id_type == "userID"


def test_get_dynamic_config(httpserver: HTTPServer):
    statsig = setup(httpserver)
    config = statsig.get_dynamic_config(StatsigUser("my_user"), "big_number")

    assert config.get_float("foo", 1) == 1e21
    assert config.get_integer("rar", 1) == 9999999999
    assert config.name == "big_number"
    assert config.rule_id == "default"
    assert config.id_type == "userID"


def test_get_experiment(httpserver: HTTPServer):
    statsig = setup(httpserver)
    experiment = statsig.get_experiment(
        StatsigUser("my_user"), "experiment_with_many_params"
    )

    assert experiment.get_string("a_string", "ERR") == "test_2"
    assert experiment.name == "experiment_with_many_params"
    assert experiment.rule_id == "7kGqFczL8Ztc2vv3tWGmvO"
    assert experiment.id_type == "userID"
    assert experiment.group_name == "Test #2"


def test_get_layer(httpserver: HTTPServer):
    statsig = setup(httpserver)
    layer = statsig.get_layer(StatsigUser("my_user"), "layer_with_many_params")

    assert layer.get_string("a_string", "ERR") == "test_2"
    assert layer.name == "layer_with_many_params"
    assert layer.rule_id == "7kGqFczL8Ztc2vv3tWGmvO"


def test_gcir(httpserver: HTTPServer):
    statsig = setup(httpserver)

    response_data = statsig.get_client_init_response(StatsigUser("my_user"))
    response = json.loads(response_data)

    assert len(response["feature_gates"]) > 0
    assert len(response["dynamic_configs"]) > 0
    assert len(response["layer_configs"]) > 0
