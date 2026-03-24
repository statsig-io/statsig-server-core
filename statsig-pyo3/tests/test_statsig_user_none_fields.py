from statsig_python_core import StatsigUser


def test_none_entries_in_constructor_dicts_are_preserved():
    user = StatsigUser(
        user_id="user_123",
        custom={"keep": "value", "keep_none": None},
        private_attributes={"secret": None},
    )

    assert user.custom == {"keep": "value", "keep_none": None}
    assert user.private_attributes == {"secret": None}


def test_none_entries_in_constructor_lists_and_nested_dicts_are_preserved():
    user = StatsigUser(
        user_id="user_123",
        custom={
            "list_with_none": ["value", None, 2],
            "dict_with_none": {"inner": None, "keep": "value"},
        },
    )

    assert user.custom == {
        "list_with_none": ["value", None, 2],
        "dict_with_none": {"inner": None, "keep": "value"},
    }
