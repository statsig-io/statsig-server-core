import json

from statsig_python_core import StatsigUser

def test_statsig_user_custom_fields_are_json_serializable():
    user = StatsigUser(
        user_id="user_123",
        custom={
            "str": "hello",
            "int": 123,
            "big_int": 9999999999,
            "float": 1e21,
            "bool": True,
            "arr": [True, "2", 3, 4.0],
            "nested": {"k1": "v1", "k2": 42},
        },
        custom_ids={"email": "test@example.com", "companyID": "123"},
        private_attributes={"ip": "1.2.3.4", "pii": {"email": "hidden@example.com"}},
    )

    # StatsigUser itself isn't guaranteed to be JSON serializable,
    # but its public dict fields should be.
    for field in [user.custom, user.custom_ids, user.private_attributes]:
        json_str = json.dumps(field)
        parsed = json.loads(json_str)
        assert parsed == field
