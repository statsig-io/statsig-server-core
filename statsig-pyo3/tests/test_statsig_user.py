import pytest
from statsig_python_core import StatsigUser

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

def test_create_user_with_custom_empty():
    user = StatsigUser(
        user_id="123",
        custom_ids={"companyID":"","stableID":""},
    )

    assert user.user_id == "123"
    assert user.custom_ids == {"companyID":"","stableID":""}

def test_create_user_with_invalid_dict():
    """Test passing an invalid dictionary type (should not raise an error)"""
    user = StatsigUser("user_123", custom={123: "haha"})
    assert user.custom == {123: "haha"} # This is actually not allowed
