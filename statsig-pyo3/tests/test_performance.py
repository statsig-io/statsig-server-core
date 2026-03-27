from statsig_python_core import Statsig, StatsigOptions, StatsigUser
from pytest_httpserver import HTTPServer
import json
from utils import get_test_data_resource
import pytest
from profile_util import profile

user = StatsigUser("a-user")


@pytest.fixture
def statsig_setup(httpserver: HTTPServer):
    dcs_content = get_test_data_resource("perf_proj_dcs.json")
    json_data = json.loads(dcs_content)

    httpserver.expect_request(
        "/v2/download_config_specs/secret-key.json"
    ).respond_with_json(json_data)

    httpserver.expect_request("/v1/log_event").respond_with_json({"success": True})

    options = StatsigOptions(
        specs_url=httpserver.url_for("/v2/download_config_specs"),
        log_event_url=httpserver.url_for("/v1/log_event"),
        # output_log_level="debug",
    )
    statsig = Statsig("secret-key", options)

    statsig.initialize().wait()

    data = {"foo": "bar"}
    baseline = profile(lambda: data.get("foo"))

    yield statsig, baseline

    # Teardown
    statsig.shutdown().wait()


def test_get_feature_gate(statsig_setup):
    statsig, baseline = statsig_setup

    results = profile(lambda: statsig.get_feature_gate(user, "test_public"))

    _config = results["value"]
    del results["value"]

    print(json.dumps(results, indent=2))
    assert (results.get("overall") / baseline.get("overall")) < 1000


def test_get_dynamic_config(statsig_setup):
    statsig, baseline = statsig_setup

    results = profile(lambda: statsig.get_dynamic_config(user, "big_dc_base64"))

    _config = results["value"]
    del results["value"]

    print(json.dumps(results, indent=2))
    assert (results.get("overall") / baseline.get("overall")) < 1000


def test_get_dynamic_config_get_value(statsig_setup):
    statsig, baseline = statsig_setup

    config = statsig.get_dynamic_config(user, "big_dc_base64")

    results = profile(lambda: config.get_string("a", "err"))

    value = results["value"]
    del results["value"]
    print(json.dumps(results, indent=2))

    assert value != "err"
    assert (results.get("overall") / baseline.get("overall")) < 1000


def test_get_dynamic_config_get_string_correctness(statsig_setup):
    statsig, _baseline = statsig_setup

    config = statsig.get_dynamic_config(user, "big_dc_base64")

    value = config.get_string("a", "err")

    assert value.startswith("TG9yZW0gaXBzdW0gZG9sb3Igc2l0IGF")


def test_get_experiment(statsig_setup):
    statsig, _baseline = statsig_setup

    results = profile(
        lambda: statsig.get_experiment(user, "another_big_expemeriment_nine")
    )

    _config = results["value"]
    del results["value"]

    print(json.dumps(results, indent=2))


def test_get_experiment_get_value(statsig_setup):
    statsig, baseline = statsig_setup

    experiment = statsig.get_experiment(user, "another_big_expemeriment_nine")

    results = profile(
        lambda: experiment.get_string("kanto_pokedex_json", "err"), iterations=10_000
    )

    value = results["value"]
    del results["value"]
    print(json.dumps(results, indent=2))

    assert value != "err"
    assert (results.get("overall") / baseline.get("overall")) < 1000


def test_get_layer(statsig_setup):
    statsig, _baseline = statsig_setup

    results = profile(lambda: statsig.get_layer(user, "big_layer"), iterations=10_000)

    _config = results["value"]
    del results["value"]

    print(json.dumps(results, indent=2))


def test_get_layer_get_value(statsig_setup):
    statsig, baseline = statsig_setup

    layer = statsig.get_layer(user, "big_layer")

    results = profile(lambda: layer.get("another_object", dict(err=1)))

    value = results["value"]
    del results["value"]
    print(json.dumps(results, indent=2))

    assert value != dict(err=1)
    assert (results.get("overall") / baseline.get("overall")) < 1000
