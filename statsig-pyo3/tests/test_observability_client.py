import json
from time import sleep
import time
from typing import Optional, Dict, List, Tuple, Any

import pytest
from pytest_httpserver import HTTPServer

from statsig_python_core import ObservabilityClient, StatsigOptions, StatsigUser, Statsig
from utils import get_test_data_resource


class MockObservabilityClient(ObservabilityClient):
    init_called = False
    dist_called = False
    error_called = False
    metrics: List[Tuple[str, str, Any, Optional[Dict[str, str]]]] = []  # Stores (type, metric_name, value, tags)

    def init(self) -> None:
        self.init_called = True
        print("Initializing ExampleObservationClient")

    def increment(self, metric_name: str, value: int = 1, tags: Optional[Dict[str, str]] = None) -> None:
        print(f"Incrementing {metric_name} by {value} with tags {tags}")
        self.metrics.append(("increment", metric_name, value, tags))

    def gauge(self, metric_name: str, value: float, tags: Optional[Dict[str, str]] = None) -> None:
        print(f"Gauging {metric_name} by {value} with tags {tags}")
        self.metrics.append(("gauge", metric_name, value, tags))

    def dist(self, metric_name: str, value: float, tags: Optional[Dict[str, str]] = None) -> None:
        print(f"Distribution {metric_name} by {value} with tags {tags}")
        self.dist_called = True
        self.metrics.append(("distribution", metric_name, value, tags))

    def error(self, tag: str, error: str) -> None:
        print(f"Error callback for {tag}: {error}")
        self.error_called = True
        self.metrics.append(("error", tag, error, None))
    
    def should_enable_high_cardinality_for_this_tag(self, tag):
        return True


@pytest.fixture
def statsig_setup(httpserver: HTTPServer):
    dcs_content = get_test_data_resource("eval_proj_dcs.json")
    json_data = json.loads(dcs_content)
    json_data["time"] = json_data["time"]
    httpserver.expect_request(
        "/v2/download_config_specs/secret-key.json"
    ).respond_with_json(json_data)

    httpserver.expect_request("/v1/log_event").respond_with_json({"success": True})

    observability_client = MockObservabilityClient()

    options = StatsigOptions()
    options.specs_url = httpserver.url_for("/v2/download_config_specs")
    options.log_event_url = httpserver.url_for("/v1/log_event")
    options.observability_client = observability_client
    options.specs_sync_interval_ms = 1
    options.output_log_level = "error"
    statsig = Statsig("secret-key", options)

    statsig.initialize().wait()

    yield statsig, observability_client, httpserver

    statsig.shutdown().wait()


def test_observability_client_usage(statsig_setup):
    """Test that MockObservabilityClient correctly tracks init(), dist() calls."""
    statsig, observability_client, httpserver = statsig_setup
    user = StatsigUser(user_id="test-user")

    statsig.check_gate(user, "test-gate")

    statsig.flush_events().wait()

    assert observability_client.init_called, "init() should have been called"

    dist_event = next(
        (m for m in observability_client.metrics if m[0] == "distribution" and m[1] == "statsig.sdk.initialization"),
        None
    )
    assert dist_event is not None, "distribution() should have been called"
    assert isinstance(dist_event[2], float)
    assert dist_event[3] == {"success": "true", "store_populated": "true", "source": "Network", "spec_source_api": f"http://{httpserver.host}:{httpserver.port}"}


def test_error_callback_usage():
    """Test that error_callback() is called."""
    observability_client = MockObservabilityClient()

    options = StatsigOptions(
        observability_client=observability_client,
    )
    statsig = Statsig("secret-key", options)
    statsig.initialize().wait()
    sleep(0.2)

    error_event = next(
        (m for m in observability_client.metrics if m[0] == "error"),
        None
    )

    assert len(observability_client.metrics) >= 3
    assert error_event is not None, "error_callback() should have been called"
    assert isinstance(error_event[2], str)

    statsig.shutdown().wait()

def test_metric_with_high_card(statsig_setup):
    """Test that MockObservabilityClient correctly tracks init(), dist() calls."""
    statsig, observability_client, _ = statsig_setup
    user = StatsigUser(user_id="test-user")

    statsig.check_gate(user, "test-gate")

    statsig.flush_events().wait()

    assert observability_client.init_called, "init() should have been called"
    time.sleep(3)

    dist_event = next(
        (m for m in observability_client.metrics if m[0] == "distribution" and m[1] == "statsig.sdk.config_propagation_diff"),
        None
    )
    assert dist_event is not None, "distribution() should have been called"
    assert isinstance(dist_event[2], float)
    assert isinstance(int(dist_event[3].get("lcut")), (int, float))
    assert isinstance(int(dist_event[3].get("prev_lcut")), (int, float))
