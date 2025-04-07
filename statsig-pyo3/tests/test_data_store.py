import json
from time import sleep
from typing import Optional
import asyncio

from statsig_python_core import DataStore, StatsigOptions, StatsigUser, Statsig, DataStoreResponse
import pytest
from pytest_httpserver import HTTPServer

from utils import get_test_data_resource

dcs_content = get_test_data_resource("eval_proj_dcs.json")
json_data = json.loads(dcs_content)

updated_dcs_json_data = json_data.copy()
if 'time' in updated_dcs_json_data:
    updated_dcs_json_data['time'] += 10

should_poll = False


class MockDataStore(DataStore):
    init_called = False
    content_set = None
    get_called_count = 0

    def initialize(self):
        self.init_called = True
        print("Initializing MockDataStore")

    def shutdown(self):
        print("Shutting down MockDataStore")

    def get(self, key: str) -> Optional[DataStoreResponse]:
        print(f"Getting value for key: {key}")
        self.get_called_count += 1
        return DataStoreResponse(
            result=dcs_content,
            time=1234567890
        )

    def set(self, key: str, value: str, time: Optional[int] = None):
        self.content_set = value
        print(f"Setting value for key: {key}")

    def support_polling_updates_for(self, key: str) -> bool:
        print(f"Checking if polling updates are supported for key: {key}: should_poll={should_poll}")
        return should_poll


@pytest.mark.asyncio
def test_data_store_usage_get(httpserver: HTTPServer):
    global should_poll
    should_poll = True
    data_store = MockDataStore()

    httpserver.expect_request(
        "/v2/download_config_specs/secret-key.json"
    ).respond_with_json({})
    httpserver.expect_request("/v1/log_event").respond_with_json({"success": True})

    options = StatsigOptions(
        specs_url=httpserver.url_for("/v2/download_config_specs"),
        log_event_url=httpserver.url_for("/v1/log_event"),
        data_store=data_store,
        specs_sync_interval_ms=1,
    )

    statsig = Statsig("secret-key", options)
    statsig.initialize().wait()

    user = StatsigUser(user_id="test_user_id")
    gate = statsig.get_feature_gate(user, "test_public")

    statsig.shutdown().wait()

    assert data_store.init_called
    assert gate.details.reason == "Adapter(DataStore):Recognized"
    assert gate.value == True
    assert gate.details.lcut == 1729873603830
    assert data_store.get_called_count > 1


def test_data_store_usage_set(httpserver: HTTPServer):
    global should_poll
    should_poll = False
    data_store = MockDataStore()

    httpserver.expect_request(
        "/v2/download_config_specs/secret-key.json"
    ).respond_with_json(updated_dcs_json_data)
    httpserver.expect_request("/v1/log_event").respond_with_json({"success": True})

    options = StatsigOptions(
        specs_url=httpserver.url_for("/v2/download_config_specs"),
        log_event_url=httpserver.url_for("/v1/log_event"),
        data_store=data_store,
        specs_sync_interval_ms=1,
    )

    statsig = Statsig("secret-key", options)
    statsig.initialize().wait()
    user = StatsigUser(user_id="test_user_id")
    gate = statsig.get_feature_gate(user, "test_public")
    assert data_store.init_called
    assert gate.details.reason == "Adapter(DataStore):Recognized"
    sleep(1)
    gate_after = statsig.get_feature_gate(user, "test_public")
    statsig.shutdown().wait()

    assert gate_after.value == True
    assert gate_after.details.lcut == 1729873603840
    assert data_store.get_called_count == 1
    assert data_store.content_set is not None
    assert json.loads(data_store.content_set) == updated_dcs_json_data
