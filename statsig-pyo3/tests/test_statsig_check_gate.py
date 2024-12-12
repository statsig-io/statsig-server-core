from sigstat_python_core import Statsig, StatsigOptions, StatsigUser
from pytest_httpserver import HTTPServer
import os
import json


def test_check_gate(httpserver: HTTPServer):
    ROOT_DIR = os.path.dirname(os.path.abspath(__file__))
    with open(
        os.path.join(ROOT_DIR, "../../statsig-lib/tests/data/eval_proj_dcs.json"), "r"
    ) as file:
        file_content = file.read()

    json_data = json.loads(file_content)

    httpserver.expect_request(
        "/v2/download_config_specs/secret-key.json"
    ).respond_with_json(json_data)

    url = httpserver.url_for("/v2/download_config_specs")

    options = StatsigOptions(url)
    statsig = Statsig("secret-key", options)

    statsig.initialize().wait()

    assert statsig.check_gate("test_public", StatsigUser("a-user"))
