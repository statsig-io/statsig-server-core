from mock_scrapi import MockScrapi
from pytest_httpserver import HTTPServer
from utils import get_test_data_resource
import subprocess
import os


def setup_server(httpserver: HTTPServer):
    mock_scrapi = MockScrapi(httpserver)
    mock_scrapi.stub("/v1/log_event", response='{"success": true}', method="POST")
    dcs_content = get_test_data_resource("eval_proj_dcs.json")
    mock_scrapi.stub(
        "/v2/download_config_specs/secret-forking-test.json",
        response=dcs_content,
        method="GET",
    )
    mock_scrapi.stub("/v1/get_id_lists", response="{}", method="POST")

    return mock_scrapi


def test_forking(httpserver: HTTPServer):
    mock_scrapi = setup_server(httpserver)

    specs_url = mock_scrapi.url_for_endpoint("/v2/download_config_specs")
    log_event_url = mock_scrapi.url_for_endpoint("/v1/log_event")
    id_lists_url = mock_scrapi.url_for_endpoint("/v1/get_id_lists")

    command = f"python tests/fork_runner.py {specs_url} {log_event_url} {id_lists_url}"
    print("Running command: ", command)
    proc = subprocess.Popen(
        command,
        shell=True,
        universal_newlines=True,
        env={**os.environ, "RUST_BACKTRACE": "full"},
    )
    try:
        proc.communicate(timeout=10)
    except subprocess.TimeoutExpired:
        proc.terminate()
        proc.wait()

    assert proc.returncode == 0
