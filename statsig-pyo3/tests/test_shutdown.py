from statsig_python_core import Statsig, StatsigOptions, StatsigUser
from mock_scrapi import MockScrapi
from utils import get_test_data_resource
import threading
import time
from pytest_httpserver import HTTPServer


def wait_ms(duration_ms):
    start_time = time.time()
    while time.time() - start_time < duration_ms / 1000.0:
        time.sleep(0.001)


def test_cycling(httpserver: HTTPServer):
    mock_scrapi = MockScrapi(httpserver)
    mock_scrapi.stub("/v1/log_event", response='{"success": true}', method="POST")
    dcs_content = get_test_data_resource("eval_proj_dcs.json")
    mock_scrapi.stub(
        "/v2/download_config_specs/secret-key.json", response=dcs_content, method="GET"
    )

    def run_statsig(inner_mock_scrapi: MockScrapi):
        options = StatsigOptions()
        options.specs_url = inner_mock_scrapi.url_for_endpoint(
            "/v2/download_config_specs"
        )
        options.log_event_url = inner_mock_scrapi.url_for_endpoint("/v1/log_event")
        options.output_log_level = "none"

        print("running statsig")
        statsig = Statsig("secret-key", options)

        print("initializing statsig")
        statsig.initialize().wait(timeout=1)
        print("initialized statsig")
        for i in range(1111):
            statsig.check_gate(StatsigUser("user-{}".format(i)), "test_public")
        print("shutting down statsig")
        statsig.shutdown().wait(timeout=1)

    threads = []

    for _ in range(3):
        t = threading.Thread(target=run_statsig, args=(mock_scrapi,))
        threads.append(t)
        t.start()

    for t in threads:
        t.join(timeout=1)

    events = mock_scrapi.get_logged_events()
    assert len(events) == 3333


def test_bg_tasks_shutdown(httpserver: HTTPServer):
    mock_scrapi = MockScrapi(httpserver)
    mock_scrapi.stub("/v1/log_event", response="{}", method="POST", status=401)
    mock_scrapi.stub("/v1/get_id_lists", response="{}", method="POST", status=401)
    mock_scrapi.stub(
        "/v2/download_config_specs/secret-key.json",
        response="{}",
        method="GET",
        status=401,
    )

    options = StatsigOptions()
    options.output_log_level = "none"
    options.specs_url = mock_scrapi.url_for_endpoint("/v2/download_config_specs")
    options.specs_sync_interval_ms = 1
    options.disable_user_agent_parsing = True
    options.disable_country_lookup = True
    options.log_event_url = mock_scrapi.url_for_endpoint("/v1/log_event")
    options.event_logging_flush_interval_ms = 1

    options.enable_id_lists = True
    options.id_lists_url = mock_scrapi.url_for_endpoint("/v1/get_id_lists")
    options.id_lists_sync_interval_ms = 1

    statsig = Statsig("secret-key", options)
    statsig.initialize().wait(timeout=1)
    statsig.shutdown().wait(timeout=1)

    wait_ms(100)
    mock_scrapi.reset()
    wait_ms(100)

    assert len(mock_scrapi.get_requests()) == 0
