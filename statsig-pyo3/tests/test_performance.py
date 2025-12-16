from statsig_python_core import Statsig, StatsigOptions, StatsigUser
from pytest_httpserver import HTTPServer
import json
from utils import get_test_data_resource
import pytest
import time

iterations = 100_000
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


def test_get_dynamic_config(statsig_setup):
    statsig, baseline = statsig_setup

    results = profile(lambda: statsig.get_dynamic_config(user, "big_dc_base64"))

    _config = results["value"]
    del results["value"]

    print(json.dumps(results, indent=2))

    assert (results.get("overall") / baseline.get("overall")) < 1000


def test_get_dynamic_config_get_value(statsig_setup):
    statsig, _baseline = statsig_setup

    config = statsig.get_dynamic_config(user, "big_dc_base64")

    results = profile(lambda: config.get_string("a", "err"))

    value = results["value"]
    del results["value"]
    print(json.dumps(results, indent=2))

    assert value is not "err"


def test_getting_the_correct_value(statsig_setup):
    statsig, _baseline = statsig_setup

    config = statsig.get_dynamic_config(user, "big_dc_base64")

    value = config.get_string("a", "err")

    assert value.startswith("TG9yZW0gaXBzdW0gZG9sb3Igc2l0IGF")


def profile(action):
    overall_start = time.perf_counter()
    durations = []

    value = None

    for _ in range(iterations):
        start = time.perf_counter()
        value = action()
        end = time.perf_counter()
        durations.append((end - start) * 1000)

    overall_end = time.perf_counter()
    overall = (overall_end - overall_start) * 1000
    durations.sort()

    p99_index = int(iterations * 0.99)

    p99 = durations[p99_index]
    min = durations[0]
    max = durations[-1]

    return {
        "value": value,
        "overall": overall,
        "p99": p99,
        "min": min,
        "max": max,
    }
