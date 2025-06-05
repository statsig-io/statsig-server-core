import pickle
import pytest
from statsig_python_core import StatsigUser


def test_statsig_user_pickle():
    # Create a StatsigUser with various fields
    original_user = StatsigUser(
        user_id="test_user_123",
        email="test@example.com",
        ip="192.168.1.1",
        country="US",
        locale="en-US",
        app_version="1.0.0",
        user_agent="Mozilla/5.0",
        custom={"premium": True, "score": 100},
        custom_ids={"email": "test@example.com", "google": "test@gmail.com"},
        private_attributes={"ip": "192.168.1.1"},
    )

    # Pickle the user
    pickled_user = pickle.dumps(original_user)

    # Unpickle the user
    unpickled_user = pickle.loads(pickled_user)

    # Verify all fields are preserved
    assert unpickled_user.user_id == original_user.user_id
    assert unpickled_user.email == original_user.email
    assert unpickled_user.ip == original_user.ip
    assert unpickled_user.country == original_user.country
    assert unpickled_user.locale == original_user.locale
    assert unpickled_user.app_version == original_user.app_version
    assert unpickled_user.user_agent == original_user.user_agent
    assert unpickled_user.custom == original_user.custom
    assert unpickled_user.custom_ids == original_user.custom_ids

    # Do Not Serialize Private Attributes
    assert unpickled_user.private_attributes == None


def test_statsig_user_pickle_minimal():
    # Test with minimal fields
    original_user = StatsigUser(user_id="test_user_123")

    # Pickle the user
    pickled_user = pickle.dumps(original_user)

    # Unpickle the user
    unpickled_user = pickle.loads(pickled_user)

    # Verify fields are preserved
    assert unpickled_user.user_id == original_user.user_id
    assert unpickled_user.email is None
    assert unpickled_user.custom is None
    assert unpickled_user.private_attributes is None
    assert unpickled_user.custom_ids == {}
