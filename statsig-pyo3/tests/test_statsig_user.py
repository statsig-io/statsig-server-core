import pytest
from statsig_python_core import StatsigUser, StatsigOptions, Statsig
import json
from typing import Mapping
from utils import get_test_data_resource
from pytest_httpserver import HTTPServer
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
        user_agent="Mozilla/5.0",
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


def test_various_custom_attr_types():
    user = StatsigUser(
        user_id="user_123",
        custom={"age": 30, "percent": 0.5, "premium": True, "arr": [True, "2", 3, 4.0]},
    )

    custom = user.custom

    assert custom["age"] == 30
    assert isinstance(custom["age"], int)
    assert custom["percent"] == 0.5
    assert isinstance(custom["percent"], float)
    assert custom["premium"] == True
    assert isinstance(custom["premium"], bool)
    assert custom["arr"] == [True, "2", 3, 4.0]
    assert isinstance(custom["arr"], list)

    assert isinstance(custom["arr"][0], bool)
    assert isinstance(custom["arr"][1], str)
    assert isinstance(custom["arr"][2], int)
    assert isinstance(custom["arr"][3], float)


def test_create_user_with_mapping():
    """Test creating a user where `custom` and `private_attributes` are `Mapping` instead of `Dict`."""
    custom_mapping: Mapping[str, int] = {"id": 30, "premium": 1}
    private_mapping: Mapping[str, str] = {"ip": "1.2.3.4"}

    user = StatsigUser(
        user_id="user_123", custom=custom_mapping, private_attributes=private_mapping
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
        private_attributes={"pii": {"email": "hidden@example.com"}},
    )

    # Nested dicts are not supported, so they are converted to empty strings
    assert user.custom == {"nested": ""}
    assert user.private_attributes == {"pii": ""}


def test_create_user_with_py_list():
    strings = StatsigUser(user_id="user_123", custom={"arr": ["one", "two", "three"]})
    assert strings.custom == {"arr": ["one", "two", "three"]}

    nums = StatsigUser(user_id="user_123", custom={"arr": [1.0, 2.0, 3.0]})
    assert nums.custom == {"arr": [1.0, 2.0, 3.0]}

    mixed = StatsigUser(
        user_id="user_123", custom={"arr": ["one", "two", "three", 1, 3]}
    )
    assert mixed.custom == {"arr": ["one", "two", "three", 1, 3]}


def test_create_user_with_both_null_user_id_and_custom_id():
    user = StatsigUser()
    assert user.user_id == ""
    assert user.custom_ids == {}


def test_create_user_with_custom_empty():
    user = StatsigUser(
        user_id="123",
        custom_ids={"companyID": "", "stableID": ""},
    )

    assert user.user_id == "123"
    assert user.custom_ids == {"companyID": "", "stableID": ""}


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


@pytest.mark.parametrize(
    "attribute,new_value",
    [
        ("email", "user@statsig.com"),
        ("ip", "1.2.3.4"),
        ("country", "US"),
        ("locale", "en-US"),
        ("app_version", "1.0.0"),
        ("user_agent", "Mozilla/5.0 (iPhone; iOS 16.6) Safari/604.1"),
    ],
)
def test_string_attribute_getter_setter(attribute, new_value):
    user = StatsigUser("")

    # Equivalent to var assignment: user.email = 'user@statsig.com'

    # Set new value
    setattr(user, attribute, new_value)
    assert getattr(user, attribute) == new_value

    # Set to None
    setattr(user, attribute, None)
    assert getattr(user, attribute) is None


@pytest.mark.parametrize(
    "attribute,new_value",
    [
        (
            "custom",
            {
                "key": "value",
                "key2": 123,
                "key3": 1.23,
                "key4": True,
                "key5": ["1", "2", "3"],
            },
        ),
        ("private_attributes", {"key": "value"}),
    ],
)
def test_map_attribute_getter_setter(attribute, new_value):
    user = StatsigUser("")

    # Equivalent to var assignment: user.custom = {'key': 'value'}

    setattr(user, attribute, new_value)
    assert getattr(user, attribute) == new_value

    # Set to None
    setattr(user, attribute, None)
    assert getattr(user, attribute) is None


def test_user_id_getter_setter():
    user = StatsigUser("")

    user.user_id = "new_user_id"
    assert user.user_id == "new_user_id"

    user.user_id = None
    assert user.user_id == ""  # defaults to empty string


def test_custom_ids_getter_setter():
    user = StatsigUser("")

    user.custom_ids = {"key": "value"}
    assert user.custom_ids == {"key": "value"}

    user.custom_ids = None

    assert isinstance(user.custom_ids, dict)
    assert len(user.custom_ids) == 0  # defaults to empty dict
