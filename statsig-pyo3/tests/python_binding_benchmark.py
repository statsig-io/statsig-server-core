import json
import os
import threading
import time
from collections import Counter
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path

from statsig_python_core import (
    DynamicConfigEvaluationOptions,
    ExperimentEvaluationOptions,
    FeatureGateEvaluationOptions,
    LayerEvaluationOptions,
    Statsig,
    StatsigOptions,
    StatsigUser,
)


DEFAULT_ITERATIONS = int(os.environ.get("STATSIG_PY_BENCH_ITERATIONS", "10000"))
SAMPLE_SIZE = 32

BENCH_GATE_NAME = "test_50_50"
BENCH_CONFIG_NAME = "operating_system_config"
BENCH_EXPERIMENT_NAME = "experiment_with_many_params"
BENCH_LAYER_NAME = "layer_with_many_params"

EXPECTED_GATE_SAMPLE = [
    "true",
    "true",
    "true",
    "false",
    "true",
    "true",
    "true",
    "true",
    "false",
    "false",
    "true",
    "false",
    "false",
    "true",
    "false",
    "false",
    "false",
    "false",
    "false",
    "false",
    "false",
    "false",
    "false",
    "true",
    "false",
    "true",
    "false",
    "false",
    "true",
    "false",
    "false",
    "false",
]
EXPECTED_CONFIG_SAMPLE = [
    "1",
    "13",
    "1",
    "13",
    "1",
    "13",
    "1",
    "13",
    "1",
    "13",
    "1",
    "13",
    "1",
    "13",
    "1",
    "13",
    "1",
    "13",
    "1",
    "13",
    "1",
    "13",
    "1",
    "13",
    "1",
    "13",
    "1",
    "13",
    "1",
    "13",
    "1",
    "13",
]
EXPECTED_EXPERIMENT_SAMPLE = [
    "test_2",
    "layer",
    "layer",
    "layer",
    "layer",
    "control",
    "layer",
    "control",
    "layer",
    "layer",
    "layer",
    "layer",
    "test_2",
    "test_2",
    "layer",
    "test_1",
    "test_2",
    "test_1",
    "test_2",
    "layer",
    "layer",
    "control",
    "layer",
    "test_2",
    "layer",
    "layer",
    "control",
    "test_1",
    "layer",
    "layer",
    "control",
    "layer",
]


def build_user(index: int) -> StatsigUser:
    email = (
        f"bench-{index}@statsig.com"
        if index % 2 == 0
        else f"bench-{index}@example.com"
    )
    return StatsigUser(user_id=f"bench-user-{index}", email=email)


class _Handler(BaseHTTPRequestHandler):
    dcs_bytes = b""

    def do_GET(self):
        path = self.path.split("?", 1)[0]
        if path.startswith("/v2/download_config_specs/") and path.endswith(".json"):
            self.send_response(200)
            self.send_header("Content-Type", "application/json")
            self.send_header("Content-Length", str(len(self.dcs_bytes)))
            self.end_headers()
            self.wfile.write(self.dcs_bytes)
            return

        self.send_response(404)
        self.end_headers()

    def do_POST(self):
        path = self.path.split("?", 1)[0]
        if path == "/v1/log_event":
            response = b'{"success":true}'
            self.send_response(200)
            self.send_header("Content-Type", "application/json")
            self.send_header("Content-Length", str(len(response)))
            self.end_headers()
            self.wfile.write(response)
            return

        self.send_response(404)
        self.end_headers()

    def log_message(self, format, *args):
        return


class LocalStatsigServer:
    def __init__(self):
        root = Path(__file__).resolve().parents[2]
        dcs_path = root / "statsig-rust" / "tests" / "data" / "eval_proj_dcs.json"
        _Handler.dcs_bytes = dcs_path.read_bytes()

        self._server = ThreadingHTTPServer(("127.0.0.1", 0), _Handler)
        self._thread = threading.Thread(target=self._server.serve_forever, daemon=True)

    @property
    def base_url(self) -> str:
        host, port = self._server.server_address
        return f"http://{host}:{port}"

    def start(self):
        self._thread.start()

    def stop(self):
        self._server.shutdown()
        self._server.server_close()
        self._thread.join()


def operation_summary(name: str, users, eval_fn):
    counts = Counter()
    sample = []
    start = time.perf_counter()

    for index, user in enumerate(users):
        result = eval_fn(user)
        counts[result] += 1
        if index < SAMPLE_SIZE:
            sample.append(result)

    elapsed = time.perf_counter() - start
    return {
        "name": name,
        "iterations": len(users),
        "total_ms": elapsed * 1000.0,
        "avg_us": elapsed * 1_000_000.0 / len(users),
        "counts": dict(sorted(counts.items())),
        "sample": sample,
    }


def cached_summary(name: str, iterations: int, eval_fn):
    start = time.perf_counter()
    last_value = None

    for _ in range(iterations):
        last_value = eval_fn()

    elapsed = time.perf_counter() - start
    return {
        "name": name,
        "iterations": iterations,
        "total_ms": elapsed * 1000.0,
        "avg_us": elapsed * 1_000_000.0 / iterations,
        "last_value": last_value,
    }


def config_signature(config) -> str:
    return str(config.get_value().get("num", 0))


def experiment_signature(experiment) -> str:
    return str(experiment.get_value().get("a_string", ""))


def layer_signature(layer) -> str:
    return str(layer.get_value().get("a_string", ""))


def assert_uncached_expectations(summary):
    operations = {item["name"]: item for item in summary["operations"]}

    assert operations["check_gate"]["counts"] == {"false": 4981, "true": 5019}
    assert operations["check_gate"]["sample"] == EXPECTED_GATE_SAMPLE

    assert operations["get_dynamic_config"]["counts"] == {"1": 5000, "13": 5000}
    assert operations["get_dynamic_config"]["sample"] == EXPECTED_CONFIG_SAMPLE

    expected_experiment_counts = {
        "control": 1615,
        "layer": 5073,
        "test_1": 1689,
        "test_2": 1623,
    }
    assert operations["get_experiment"]["counts"] == expected_experiment_counts
    assert operations["get_experiment"]["sample"] == EXPECTED_EXPERIMENT_SAMPLE

    assert operations["get_layer"]["counts"] == expected_experiment_counts
    assert operations["get_layer"]["sample"] == EXPECTED_EXPERIMENT_SAMPLE


def main():
    iterations = DEFAULT_ITERATIONS
    server = LocalStatsigServer()
    server.start()

    gate_options = FeatureGateEvaluationOptions(disable_exposure_logging=True)
    config_options = DynamicConfigEvaluationOptions(disable_exposure_logging=True)
    experiment_options = ExperimentEvaluationOptions(disable_exposure_logging=True)
    layer_options = LayerEvaluationOptions(disable_exposure_logging=True)

    options = StatsigOptions(
        specs_url=f"{server.base_url}/v2/download_config_specs",
        log_event_url=f"{server.base_url}/v1/log_event",
        disable_all_logging=True,
    )
    statsig = Statsig("secret-key", options)
    statsig.initialize().wait()

    uncached_users = [build_user(index) for index in range(iterations)]
    cached_user = build_user(0)

    experiment_raw = statsig._INTERNAL_get_experiment(
        cached_user, BENCH_EXPERIMENT_NAME, experiment_options
    )
    layer_raw = statsig._INTERNAL_get_layer(cached_user, BENCH_LAYER_NAME, layer_options)
    has_experiment_dict = hasattr(statsig, "_INTERNAL_get_experiment_as_dict")
    has_layer_exposure_dict = hasattr(statsig, "_INTERNAL_get_layer_exposure_raw_and_dict")

    layer_exposure_raw = None
    layer_dict = None
    if has_layer_exposure_dict:
        layer_exposure_raw, layer_dict = statsig._INTERNAL_get_layer_exposure_raw_and_dict(
            cached_user, BENCH_LAYER_NAME, layer_options
        )

    uncached_summary = {
        "mode": "uncached_public",
        "iterations": iterations,
        "operations": [
            operation_summary(
                "check_gate",
                uncached_users,
                lambda user: str(
                    statsig.check_gate(user, BENCH_GATE_NAME, gate_options)
                ).lower(),
            ),
            operation_summary(
                "get_dynamic_config",
                uncached_users,
                lambda user: config_signature(
                    statsig.get_dynamic_config(user, BENCH_CONFIG_NAME, config_options)
                ),
            ),
            operation_summary(
                "get_experiment",
                uncached_users,
                lambda user: experiment_signature(
                    statsig.get_experiment(
                        user, BENCH_EXPERIMENT_NAME, experiment_options
                    )
                ),
            ),
            operation_summary(
                "get_layer",
                uncached_users,
                lambda user: layer_signature(
                    statsig.get_layer(user, BENCH_LAYER_NAME, layer_options)
                ),
            ),
        ],
    }

    if iterations == 10000:
        assert_uncached_expectations(uncached_summary)

    cached_operations = [
        cached_summary(
            "check_gate",
            iterations,
            lambda: statsig.check_gate(cached_user, BENCH_GATE_NAME, gate_options),
        ),
        cached_summary(
            "get_dynamic_config_public",
            iterations,
            lambda: config_signature(
                statsig.get_dynamic_config(cached_user, BENCH_CONFIG_NAME, config_options)
            ),
        ),
        cached_summary(
            "get_dynamic_config_internal_dict",
            iterations,
            lambda: statsig._INTERNAL_get_dynamic_config_as_dict(
                cached_user, BENCH_CONFIG_NAME, config_options
            )["value"]["num"],
        ),
        cached_summary(
            "get_experiment_public",
            iterations,
            lambda: experiment_signature(
                statsig.get_experiment(cached_user, BENCH_EXPERIMENT_NAME, experiment_options)
            ),
        ),
        cached_summary(
            "get_experiment_internal_raw",
            iterations,
            lambda: statsig._INTERNAL_get_experiment(
                cached_user, BENCH_EXPERIMENT_NAME, experiment_options
            ),
        ),
    ]

    if has_experiment_dict:
        cached_operations.append(
            cached_summary(
                "get_experiment_internal_dict",
                iterations,
                lambda: statsig._INTERNAL_get_experiment_as_dict(
                    cached_user, BENCH_EXPERIMENT_NAME, experiment_options
                )["value"]["a_string"],
            )
        )

    cached_operations.append(
        cached_summary(
            "get_experiment_json_loads",
            iterations,
            lambda: json.loads(experiment_raw)["value"]["a_string"],
        )
    )

    cached_operations.extend(
        [
            cached_summary(
                "get_layer_public",
                iterations,
                lambda: layer_signature(
                    statsig.get_layer(cached_user, BENCH_LAYER_NAME, layer_options)
                ),
            ),
            cached_summary(
                "get_layer_internal_raw",
                iterations,
                lambda: statsig._INTERNAL_get_layer(
                    cached_user, BENCH_LAYER_NAME, layer_options
                ),
            ),
        ]
    )

    if has_layer_exposure_dict:
        cached_operations.extend(
            [
                cached_summary(
                    "get_layer_internal_exposure_raw_and_dict",
                    iterations,
                    lambda: statsig._INTERNAL_get_layer_exposure_raw_and_dict(
                        cached_user, BENCH_LAYER_NAME, layer_options
                    )[1]["value"]["a_string"],
                ),
                cached_summary(
                    "get_layer_exposure_raw_json_loads",
                    iterations,
                    lambda: json.loads(layer_exposure_raw)["name"],
                ),
                cached_summary(
                    "get_layer_dict_value_read",
                    iterations,
                    lambda: layer_dict["value"]["a_string"],
                ),
            ]
        )

    cached_operations.append(
        cached_summary(
            "get_layer_json_loads",
            iterations,
            lambda: json.loads(layer_raw)["value"]["a_string"],
        )
    )

    cached_summary_data = {
        "mode": "cached_breakdown",
        "iterations": iterations,
        "operations": cached_operations,
    }

    print(
        json.dumps(
            {
                "python": {
                    "version": os.sys.version,
                },
                "uncached_public": uncached_summary,
                "cached_breakdown": cached_summary_data,
            },
            indent=2,
        )
    )

    statsig.shutdown().wait()
    server.stop()


if __name__ == "__main__":
    main()
