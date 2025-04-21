import json
from typing import Mapping

import pytest
from pytest_httpserver import HTTPServer
from statsig_python_core import StatsigUser, StatsigOptions, Statsig

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
        wait_for_user_agent_init=True,
        wait_for_country_lookup_init=True,
        output_log_level="debug",
    )
    statsig = Statsig("secret-key", options)

    statsig.initialize().wait()

    yield statsig

    # Teardown
    statsig.shutdown().wait()


def test_create_user_with_only_string_fields():
    user = StatsigUser(
        user_id="user_123",
        email="test@example.com",
        ip="192.168.1.1",
        country="US",
        locale="en-US",
        app_version="1.0.0",
        user_agent="Mozilla/5.0"
    )

    assert user.user_id == "user_123"
    assert user.email == "test@example.com"
    assert user.ip == "192.168.1.1"
    assert user.country == "US"
    assert user.locale == "en-US"
    assert user.app_version == "1.0.0"
    assert user.user_agent == "Mozilla/5.0"


def test_create_user_with_custom_fields():
    user = StatsigUser(
        user_id="user_123",
        custom={"id": 30, "premium": True},
        custom_ids={"email": "whd@statsig.com", "google": "whd@gmail"},
        private_attributes={"ip": "1.2.3.4"},
    )

    assert user.custom == {"id": 30, "premium": True}
    assert user.custom_ids == {"email": "whd@statsig.com", "google": "whd@gmail"}
    assert user.private_attributes == {"ip": "1.2.3.4"}


def test_create_user_with_mapping():
    """Test creating a user where `custom` and `private_attributes` are `Mapping` instead of `Dict`."""
    custom_mapping: Mapping[str, int] = {"id": 30, "premium": 1}
    private_mapping: Mapping[str, str] = {"ip": "1.2.3.4"}

    user = StatsigUser(
        user_id="user_123",
        custom=custom_mapping,
        private_attributes=private_mapping
    )

    assert user.custom == {"id": 30, "premium": 1}
    assert user.private_attributes == {"ip": "1.2.3.4"}


def test_create_user_with_empty_dicts():
    user = StatsigUser("user_123", custom={}, custom_ids={}, private_attributes={})

    assert user.custom == {}
    assert user.custom_ids == {}
    assert user.private_attributes == {}


def test_create_user_with_nested_dict():
    user = StatsigUser(
        "user_123",
        custom={"nested": {"key1": "value1", "key2": 42}},
        private_attributes={"pii": {"email": "hidden@example.com"}}
    )

    assert user.custom == {"nested": {"key1": "value1", "key2": 42}}
    assert user.private_attributes == {"pii": {"email": "hidden@example.com"}}


def test_create_user_with_py_list():
    user = StatsigUser(
        user_id="user_123",
        custom={"arr": ["one", "two", "three", 1, 3]}
    )
    assert user.custom == {"arr": ["one", "two", "three", 1, 3]}


def test_create_user_with_both_null_user_id_and_custom_id():
    user = StatsigUser()
    assert user.user_id is None
    assert user.custom_ids is None


def test_create_user_with_custom_empty():
    user = StatsigUser(
        user_id="123",
        custom_ids={"companyID": "", "stableID": ""},
    )

    assert user.user_id == "123"
    assert user.custom_ids == {"companyID": "", "stableID": ""}


def test_create_user_with_invalid_dict():
    """Test passing an invalid dictionary type (should not raise an error)"""
    user = StatsigUser("user_123", custom={123: "haha"})
    assert user.custom == {123: "haha"}  # This is actually not allowed


def test_user_set_attribute_propagate_to_rust(statsig_setup):
    statsig = statsig_setup
    user = StatsigUser("a-user")

    user.email = "user@statsig.com"
    assert statsig.check_gate(user, "test_email") == True

    user.country = "US"
    assert statsig.check_gate(user, "test_country") == True

    user.user_agent = "Mozilla/5.0 (iPhone; iOS 16.6) Safari/604.1"
    assert statsig.check_gate(user, "test_ua") == True

    user.ip = "1.0.0.0"
    assert statsig.check_gate(user, "test_ip_field") == True

    user.custom = {
        "newUser": False,
    }
    assert statsig.check_gate(user, "test_custom") == True

    user.custom_ids = {
        "companyID": "12345",
    }
    assert statsig.check_gate(user, "test_numeric_custom_id") == True
