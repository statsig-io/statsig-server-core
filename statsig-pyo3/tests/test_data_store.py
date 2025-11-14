import json
from time import sleep
from typing import Optional
import pytest

from statsig_python_core import (
    DataStore,
    StatsigOptions,
    StatsigUser,
    Statsig,
    DataStoreResponse,
)
from pytest_httpserver import HTTPServer
from utils import get_test_data_resource

known_lcut = 1763138293896

dcs_content = get_test_data_resource("eval_proj_dcs.json")
json_data = json.loads(dcs_content)

del json_data["checksum"]

updated_dcs_json_data = json_data.copy()
if "time" in updated_dcs_json_data:
    updated_dcs_json_data["time"] += 10


class MockDataStore(DataStore):
    init_called = False
    content_set = None
    get_called_count = 0
    should_poll = False

    def __new__(cls, test_param: str = ""):
        instance = super().__new__(cls)
        instance.test_param = test_param
        return instance

    def initialize(self):
        self.init_called = True
        print("Initializing MockDataStore")

    def shutdown(self):
        print("Shutting down MockDataStore")

    def get(self, key: str) -> Optional[DataStoreResponse]:
        print(f"Getting value for key: {key}")
        self.get_called_count += 1
        return DataStoreResponse(result=dcs_content, time=1234567890)

    def set(self, key: str, value: str, time: Optional[int] = None):
        self.content_set = value
        print(f"Setting value for key: {key}")

    def support_polling_updates_for(self, key: str) -> bool:
        print(
            f"Checking if polling updates are supported for key: {key}: should_poll={self.should_poll}"
        )
        return self.should_poll


@pytest.fixture
def statsig_setup(httpserver: HTTPServer):
    data_store = MockDataStore(test_param="test_param")

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
    user = StatsigUser(user_id="test_user_id")

    yield statsig, data_store, user

    statsig.shutdown().wait()


def test_data_store_usage_get_with_test_param():
    data_store = MockDataStore(test_param="test_param")
    assert data_store.test_param == "test_param"


def test_data_store_usage_get(statsig_setup):
    statsig, data_store, user = statsig_setup
    data_store.should_poll = True
    statsig.initialize().wait()

    gate = statsig.get_feature_gate(user, "test_public")

    statsig.flush_events().wait()

    assert data_store.init_called
    assert gate.details.reason == "Adapter(DataStore):Recognized"
    assert gate.value == True
    assert gate.details.lcut == known_lcut
    assert data_store.get_called_count > 1


def test_data_store_usage_set(statsig_setup):
    statsig, data_store, user = statsig_setup
    data_store.should_poll = False
    statsig.initialize().wait()

    gate = statsig.get_feature_gate(user, "test_public")

    assert data_store.init_called
    assert gate.details.reason == "Adapter(DataStore):Recognized"
    sleep(1)

    gate_after = statsig.get_feature_gate(user, "test_public")
    statsig.flush_events().wait()

    assert gate_after.value == True
    assert gate_after.details.lcut == known_lcut + 10
    assert data_store.get_called_count == 1
    assert data_store.content_set is not None
    assert json.loads(data_store.content_set) == updated_dcs_json_data
