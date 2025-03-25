import pytest
from statsig_python_core import Statsig, StatsigOptions, StatsigUser
from mock_scrapi import MockScrapi
from utils import get_test_data_resource
from pytest_httpserver import HTTPServer


@pytest.fixture
def statsig_setup(httpserver: HTTPServer):
    mock_scrapi = MockScrapi(httpserver)
    dcs_content = get_test_data_resource("eval_proj_dcs.json")
    mock_scrapi.stub(
        "/v2/download_config_specs/secret-key.json", response=dcs_content, method="GET"
    )
    mock_scrapi.stub("/v1/log_event", response='{"success": true}', method="POST")

    options = StatsigOptions()
    options.specs_url = mock_scrapi.url_for_endpoint("/v2/download_config_specs")
    options.log_event_url = mock_scrapi.url_for_endpoint("/v1/log_event")
    options.output_log_level = "none"

    Statsig.remove_shared()

    yield options, mock_scrapi

    if Statsig.has_shared_instance():
        statsig = Statsig.shared()
        statsig.shutdown().wait()

def test_creating_shared_instance(statsig_setup):
    options, _ = statsig_setup

    statsig = Statsig.new_shared("secret-key", options)
    statsig.initialize().wait()
    assert statsig.check_gate(StatsigUser("my_user"), "test_public")

def test_getting_shared_instance(statsig_setup):
    options, _ = statsig_setup

    statsig = Statsig.new_shared("secret-key", options)
    shared_statsig = Statsig.shared()

    assert shared_statsig == statsig

    shared_statsig.initialize().wait()
    assert shared_statsig.check_gate(StatsigUser("my_user"), "test_public")

def test_removing_shared_instance(statsig_setup):
    options, _ = statsig_setup

    statsig = Statsig.new_shared("secret-key", options)
    statsig.initialize().wait()
    Statsig.remove_shared()

    shared_statsig = Statsig.shared()
    assert not shared_statsig.check_gate(StatsigUser("my_user"), "test_public")

def test_checking_if_shared_instance_exists():
    Statsig.new_shared("secret-key")
    assert Statsig.has_shared_instance()

    Statsig.remove_shared()
    assert not Statsig.has_shared_instance()
