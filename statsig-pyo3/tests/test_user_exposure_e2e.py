"""
verify the complete structure and correctness of exposure events,
But specifically for StatsigUser Correctness.
"""
import gzip
import json
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

    statsig = Statsig("secret-key", options)
    statsig.initialize().wait()

    yield statsig, mock_scrapi

    statsig.shutdown().wait()

def test_updating_user_email_via_setter(statsig_setup):
    statsig, mock_scrapi = statsig_setup
    user = StatsigUser("test_user_123", email="test@example.com", country="US")

    statsig.check_gate(user, "test_public")
    statsig.flush_events().wait()
    events = mock_scrapi.get_logged_events()

    assert len(events) == 1
    event = events[0]

    assert event["eventName"] == "statsig::gate_exposure"

    assert "user" in event
    user_data = event["user"]
    assert user_data["userID"] == "test_user_123"
    assert user_data.get("email") == "test@example.com"
    assert user_data.get("country") == "US"

    # modify the user and check the exposure event
    user.email = "test2@example.com"
    statsig.check_gate(user, "test_public")
    statsig.flush_events().wait()
    events = mock_scrapi.get_logged_events()
    assert len(events) == 2
    event = events[1]
    assert event["user"]["email"] == "test2@example.com" # should only update the email
    assert event["user"]["country"] == "US"
    assert event["user"]["userID"] == "test_user_123"

def test_gate_exposure_with_custom_user_attributes(statsig_setup):
    statsig, mock_scrapi = statsig_setup
    user = StatsigUser(
        "user_456",
        custom={"premium": True, "age": 30, "score": 95.5},
        custom_ids={"companyID": "comp_123"},
        private_attributes={"ip": "1.2.3.4"},
    )

    statsig.check_gate(user, "test_public")
    statsig.flush_events().wait()
    events = mock_scrapi.get_logged_events()

    assert len(events) == 1
    event = events[0]
    user_data = event["user"]

    assert "custom" in user_data
    custom = user_data.get("custom")
    assert custom.get("premium") is True
    assert custom.get("age") == 30
    assert custom.get("score") == 95.5

    assert "customIDs" in user_data
    custom_ids = user_data.get("customIDs")
    assert custom_ids.get("companyID") == "comp_123"

    statsig.check_gate(user, "test_public")
    statsig.flush_events().wait()

    events = mock_scrapi.get_logged_events()
    gate_exposures = [e for e in events if e["eventName"] == "statsig::gate_exposure"]

def test_exposure_with_empty_user_id(statsig_setup):
    """Test that exposures work correctly with empty user ID."""
    statsig, mock_scrapi = statsig_setup
    user = StatsigUser()  # Empty user ID

    statsig.check_gate(user, "test_public")
    statsig.flush_events().wait()

    events = mock_scrapi.get_logged_events()
    gate_exposures = [e for e in events if e["eventName"] == "statsig::gate_exposure"]

    assert len(gate_exposures) == 1
    event = gate_exposures[0]

    # User ID should be empty string or not present
    user_data = event.get("user", {})
    user_id = user_data.get("userID", "")
    assert user_id == "" or user_id is None
