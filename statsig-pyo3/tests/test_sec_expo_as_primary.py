import json

import pytest
from pytest_httpserver import HTTPServer
from statsig_python_core import Statsig, StatsigOptions, StatsigUser

from mock_scrapi import MockScrapi
from utils import get_test_data_resource

SEC_EXPO_AS_PRIMARY_FLAG = "sec_expo_as_primary:abc123"
SEC_EXPO_AS_PRIMARY_FLAG_BUCKET = 307


def _create_statsig(
    httpserver: HTTPServer, experimental_flags=None, sec_expo_number=None
):
    mock_scrapi = MockScrapi(httpserver)
    dcs_content = json.loads(get_test_data_resource("eval_proj_dcs.json"))
    if sec_expo_number is not None:
        dcs_content.setdefault("sdk_configs", {})["sec_expo_number"] = sec_expo_number

    mock_scrapi.stub(
        "/v2/download_config_specs/secret-key.json",
        response=json.dumps(dcs_content),
        method="GET",
    )
    mock_scrapi.stub("/v1/log_event", response='{"success": true}', method="POST")

    options = StatsigOptions()
    options.specs_url = mock_scrapi.url_for_endpoint("/v2/download_config_specs")
    options.log_event_url = mock_scrapi.url_for_endpoint("/v1/log_event")
    options.output_log_level = "none"
    options.experimental_flags = experimental_flags

    statsig = Statsig("secret-key", options)
    statsig.initialize().wait()

    return statsig, mock_scrapi


def _assert_exposure(event, event_name, metadata):
    assert event["eventName"] == event_name
    for key, value in metadata.items():
        assert event["metadata"][key] == value


@pytest.mark.parametrize(
    "evaluate, expected_primary, expected_secondary_events",
    [
        pytest.param(
            lambda statsig: statsig.check_gate(
                StatsigUser("a_user_id"), "test_nested_gate_condition"
            ),
            (
                "statsig::gate_exposure",
                {
                    "gate": "test_nested_gate_condition",
                    "gateValue": "true",
                    "ruleID": "6MlXHRavmo1ujM1NkZNjhQ",
                },
            ),
            [
                (
                    "statsig::gate_exposure",
                    {
                        "gate": "test_email",
                        "gateValue": "false",
                        "ruleID": "default",
                    },
                ),
                (
                    "statsig::gate_exposure",
                    {
                        "gate": "test_environment_tier",
                        "gateValue": "false",
                        "ruleID": "default",
                    },
                ),
            ],
            id="gate",
        ),
        pytest.param(
            lambda statsig: statsig.get_dynamic_config(
                StatsigUser("a_user_id"), "operating_system_config"
            ),
            (
                "statsig::config_exposure",
                {
                    "config": "operating_system_config",
                    "ruleID": "default",
                },
            ),
            [
                (
                    "statsig::gate_exposure",
                    {
                        "gate": "test_email",
                        "gateValue": "false",
                        "ruleID": "default",
                    },
                ),
            ],
            id="dynamic_config",
        ),
        pytest.param(
            lambda statsig: statsig.get_experiment(
                StatsigUser("a-user", email="daniel@statsig.com"),
                "running_exp_in_unlayered_with_holdout",
            ),
            (
                "statsig::config_exposure",
                {
                    "config": "running_exp_in_unlayered_with_holdout",
                    "ruleID": "5suobe8yyvznqasn9Ph1dI",
                },
            ),
            [
                (
                    "statsig::gate_exposure",
                    {
                        "gate": "global_holdout",
                        "gateValue": "false",
                        "ruleID": "3QoA4ncNdVGBaMt3N1KYjz:0.50:1",
                    },
                ),
                (
                    "statsig::gate_exposure",
                    {
                        "gate": "exp_holdout",
                        "gateValue": "false",
                        "ruleID": "1rEqLOpCROaRafv7ubGgax",
                    },
                ),
            ],
            id="experiment",
        ),
        pytest.param(
            lambda statsig: statsig.get_layer(
                StatsigUser("a_user_id"), "layer_in_global_holdout"
            ).get_string("shared_param", ""),
            (
                "statsig::layer_exposure",
                {
                    "config": "layer_in_global_holdout",
                    "parameterName": "shared_param",
                },
            ),
            [
                (
                    "statsig::gate_exposure",
                    {
                        "gate": "global_holdout",
                        "gateValue": "false",
                        "ruleID": "3QoA4ncNdVGBaMt3N1KYjz:0.50:1",
                    },
                ),
            ],
            id="layer",
        ),
    ],
)
def test_gate_dynamic_config_experiment_and_layer_log_secondary_exposures_as_primary(
    httpserver: HTTPServer,
    evaluate,
    expected_primary,
    expected_secondary_events,
):
    statsig, mock_scrapi = _create_statsig(
        httpserver,
        experimental_flags={SEC_EXPO_AS_PRIMARY_FLAG},
        sec_expo_number=1000,
    )

    try:
        evaluate(statsig)
        statsig.flush_events().wait()

        events = mock_scrapi.get_logged_events()
        assert len(events) == 1 + len(expected_secondary_events)

        for event in events:
            assert event["secondaryExposures"] == []

        _assert_exposure(events[0], expected_primary[0], expected_primary[1])
        for event, expected in zip(events[1:], expected_secondary_events):
            _assert_exposure(event, expected[0], expected[1])
    finally:
        statsig.shutdown().wait()


def test_secondary_exposures_logged_as_primary_when_flag_enabled(
    httpserver: HTTPServer,
):
    statsig, mock_scrapi = _create_statsig(
        httpserver,
        experimental_flags={SEC_EXPO_AS_PRIMARY_FLAG},
        sec_expo_number=1000,
    )

    try:
        assert statsig.check_gate(
            StatsigUser("a_user_id"), "test_nested_gate_condition"
        )
        statsig.flush_events().wait()

        events = mock_scrapi.get_logged_events()
        assert len(events) == 3

        for event in events:
            assert event["secondaryExposures"] == []

        _assert_exposure(
            events[0],
            "statsig::gate_exposure",
            {
                "gate": "test_nested_gate_condition",
                "gateValue": "true",
                "ruleID": "6MlXHRavmo1ujM1NkZNjhQ",
            },
        )
        _assert_exposure(
            events[1],
            "statsig::gate_exposure",
            {
                "gate": "test_email",
                "gateValue": "false",
                "ruleID": "default",
            },
        )
        _assert_exposure(
            events[2],
            "statsig::gate_exposure",
            {
                "gate": "test_environment_tier",
                "gateValue": "false",
                "ruleID": "default",
            },
        )
    finally:
        statsig.shutdown().wait()


def test_secondary_exposures_remain_on_primary_when_flag_not_enabled(
    httpserver: HTTPServer,
):
    statsig, mock_scrapi = _create_statsig(httpserver, sec_expo_number=300)

    try:
        assert statsig.check_gate(
            StatsigUser("a_user_id"), "test_nested_gate_condition"
        )
        statsig.flush_events().wait()

        events = mock_scrapi.get_logged_events()
        assert len(events) == 1

        _assert_exposure(
            events[0],
            "statsig::gate_exposure",
            {
                "gate": "test_nested_gate_condition",
                "gateValue": "true",
                "ruleID": "6MlXHRavmo1ujM1NkZNjhQ",
            },
        )
        assert events[0]["secondaryExposures"] == [
            {
                "gate": "test_email",
                "gateValue": "false",
                "ruleID": "default",
            },
            {
                "gate": "test_environment_tier",
                "gateValue": "false",
                "ruleID": "default",
            },
        ]
    finally:
        statsig.shutdown().wait()


def test_secondary_exposures_remain_on_primary_when_sec_expo_number_is_zero(
    httpserver: HTTPServer,
):
    statsig, mock_scrapi = _create_statsig(
        httpserver,
        experimental_flags={SEC_EXPO_AS_PRIMARY_FLAG},
        sec_expo_number=0,
    )

    try:
        assert statsig.check_gate(
            StatsigUser("a_user_id"), "test_nested_gate_condition"
        )
        statsig.flush_events().wait()

        events = mock_scrapi.get_logged_events()
        assert len(events) == 1

        _assert_exposure(
            events[0],
            "statsig::gate_exposure",
            {
                "gate": "test_nested_gate_condition",
                "gateValue": "true",
                "ruleID": "6MlXHRavmo1ujM1NkZNjhQ",
            },
        )
        assert events[0]["secondaryExposures"] == [
            {
                "gate": "test_email",
                "gateValue": "false",
                "ruleID": "default",
            },
            {
                "gate": "test_environment_tier",
                "gateValue": "false",
                "ruleID": "default",
            },
        ]
    finally:
        statsig.shutdown().wait()


def test_secondary_exposures_remain_on_primary_when_sec_expo_number_missing(
    httpserver: HTTPServer,
):
    statsig, mock_scrapi = _create_statsig(
        httpserver,
        experimental_flags={SEC_EXPO_AS_PRIMARY_FLAG},
    )

    try:
        assert statsig.check_gate(
            StatsigUser("a_user_id"), "test_nested_gate_condition"
        )
        statsig.flush_events().wait()

        events = mock_scrapi.get_logged_events()
        assert len(events) == 1

        _assert_exposure(
            events[0],
            "statsig::gate_exposure",
            {
                "gate": "test_nested_gate_condition",
                "gateValue": "true",
                "ruleID": "6MlXHRavmo1ujM1NkZNjhQ",
            },
        )
        assert events[0]["secondaryExposures"] == [
            {
                "gate": "test_email",
                "gateValue": "false",
                "ruleID": "default",
            },
            {
                "gate": "test_environment_tier",
                "gateValue": "false",
                "ruleID": "default",
            },
        ]
    finally:
        statsig.shutdown().wait()


@pytest.mark.parametrize(
    "sec_expo_number, should_log_as_primary",
    [
        (0, False),
        (SEC_EXPO_AS_PRIMARY_FLAG_BUCKET, False),
        (SEC_EXPO_AS_PRIMARY_FLAG_BUCKET + 1, True),
        (1000, True),
    ],
)
def test_secondary_exposures_roll_out_from_zero_to_thousand(
    httpserver: HTTPServer,
    sec_expo_number,
    should_log_as_primary,
):
    statsig, mock_scrapi = _create_statsig(
        httpserver,
        experimental_flags={SEC_EXPO_AS_PRIMARY_FLAG},
        sec_expo_number=sec_expo_number,
    )

    try:
        assert statsig.check_gate(
            StatsigUser("a_user_id"), "test_nested_gate_condition"
        )
        statsig.flush_events().wait()

        events = mock_scrapi.get_logged_events()
        if should_log_as_primary:
            assert len(events) == 3
            for event in events:
                assert event["secondaryExposures"] == []
        else:
            assert len(events) == 1
            assert len(events[0]["secondaryExposures"]) == 2
    finally:
        statsig.shutdown().wait()
