import asyncio
import random
import string
import time
from typing import List, Dict, Any, Callable, Optional
import os
from statsig import (
    statsig,
    StatsigUser,
    StatsigOptions,
    HashingAlgorithm,
    StatsigServer,
)
from importlib.metadata import version
import json

sdk_type = "py-server"
sdk_version = version("statsig")

SCRAPI_URL = "http://scrapi:8000"
ITER_ULTRA_LITE = 100
ITER_LITE = 1000
ITER_HEAVY = 10_000


class BenchmarkResult:
    def __init__(
        self,
        benchmark_name: str,
        p99: float,
        max: float,
        min: float,
        median: float,
        avg: float,
        spec_name: str,
        sdk_type: str,
        sdk_version: str,
    ):
        self.benchmark_name = benchmark_name
        self.p99 = p99
        self.max = max
        self.min = min
        self.median = median
        self.avg = avg
        self.spec_name = spec_name
        self.sdk_type = sdk_type
        self.sdk_version = sdk_version

    def to_dict(self) -> Dict[str, Any]:
        return {
            "benchmarkName": self.benchmark_name,
            "p99": self.p99,
            "max": self.max,
            "min": self.min,
            "median": self.median,
            "avg": self.avg,
            "specName": self.spec_name,
            "sdkType": self.sdk_type,
            "sdkVersion": self.sdk_version,
        }


def try_load_file(path: str) -> Any:
    for i in range(10):
        if os.path.exists(path):
            break
        time.sleep(1)

    with open(path, "r") as f:
        return json.load(f)


def setup():
    # Wait for spec_names.json to be available
    spec_names = try_load_file("/shared-volume/spec_names.json")
    large_proj_spec_names = try_load_file("/shared-volume/large_proj_spec_names.json")

    return {
        "spec_names": spec_names,
        "large_proj_spec_names": large_proj_spec_names,
    }


def create_user() -> StatsigUser:
    user_id = "".join(random.choices(string.ascii_lowercase + string.digits, k=13))

    return StatsigUser(
        user_id=user_id,
        email="user@example.com",
        ip="127.0.0.1",
        locale="en-US",
        app_version="1.0.0",
        country="US",
        user_agent="Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36",
        custom={"isAdmin": False},
        private_attributes={"isPaid": "nah"},
    )


def create_user_with_benchmark_payload() -> StatsigUser:
    return StatsigUser(
        user_id="a_user_id",
        app_version="1.0.0",
        custom_ids={
            "custom_id": "a_long_custom_id_value_goes_here",
            "employee_id": "456",
        },
        private_attributes={
            "private_attr": "secret",
            "private_array": [1, 2, 3],
            "private_object": {"key": "value"},
        },
        custom={
            "custom_attr": "custom_value",
            "custom_array": [1, 2, 3],
            "custom_object": {"key": "value"},
            "custom_number": 123,
            "custom_boolean": True,
            "large_custom_string": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        },
        ip="127.0.0.1",
        country="US",
        email="test@test.com",
        user_agent="Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/123.0.0.0 Safari/537.36",
        locale="en_US",
    )


def assert_cond(condition: bool, message: str):
    if not condition:
        raise Exception(message)


async def benchmark(
    bench_name: str,
    spec_name: str,
    iterations: int,
    func: Callable,
    results: List[BenchmarkResult],
    cleanup: Optional[Callable] = None,
):
    durations = []

    for _ in range(iterations):
        start = time.perf_counter()
        result = func()
        end = time.perf_counter()
        durations.append((end - start) * 1000)
        if cleanup:
            cleanup(result)

    # Calculate p99
    durations.sort()
    p99_index = int(iterations * 0.99)

    result = BenchmarkResult(
        benchmark_name=bench_name,
        p99=durations[p99_index],
        max=durations[-1],
        min=durations[0],
        median=durations[len(durations) // 2],
        avg=sum(durations) / len(durations),
        spec_name=spec_name,
        sdk_type=sdk_type,
        sdk_version=sdk_version,
    )

    results.append(result)

    print(
        f"{bench_name:<30} "
        f"p99({result.p99:.4f}ms){'':<15} "
        f"max({result.max:.4f}ms){'':<15} "
        f"{spec_name}"
    )

    await asyncio.sleep(0.001)  # 1ms delay


async def benchmark_feature_gates(
    global_user: StatsigUser,
    statsig_server: Any,
    spec_names: Dict[str, List[str]],
    results: List[BenchmarkResult],
):
    for gate in spec_names["feature_gates"]:
        await benchmark(
            "check_gate",
            gate,
            ITER_HEAVY,
            lambda g=gate: statsig_server.check_gate(create_user(), g),
            results,
        )

        await benchmark(
            "check_gate_global_user",
            gate,
            ITER_HEAVY,
            lambda g=gate: statsig_server.check_gate(global_user, g),
            results,
        )

        await benchmark(
            "get_feature_gate",
            gate,
            ITER_HEAVY,
            lambda g=gate: statsig_server.get_feature_gate(create_user(), g),
            results,
        )

        await benchmark(
            "get_feature_gate_global_user",
            gate,
            ITER_HEAVY,
            lambda g=gate: statsig_server.get_feature_gate(global_user, g),
            results,
        )


async def benchmark_dynamic_configs(
    global_user: StatsigUser,
    statsig_server: Any,
    spec_names: Dict[str, List[str]],
    results: List[BenchmarkResult],
):
    for config in spec_names["dynamic_configs"]:
        await benchmark(
            "get_dynamic_config",
            config,
            ITER_HEAVY,
            lambda c=config: statsig_server.get_config(create_user(), c),
            results,
        )

        await benchmark(
            "get_dynamic_config_global_user",
            config,
            ITER_HEAVY,
            lambda c=config: statsig_server.get_config(global_user, c),
            results,
        )


async def benchmark_experiments(
    global_user: StatsigUser,
    statsig_server: Any,
    spec_names: Dict[str, List[str]],
    results: List[BenchmarkResult],
):
    for experiment in spec_names["experiments"]:
        await benchmark(
            "get_experiment",
            experiment,
            ITER_HEAVY,
            lambda e=experiment: statsig_server.get_experiment(global_user, e),
            results,
        )


async def benchmark_layers(
    global_user: StatsigUser,
    statsig_server: Any,
    spec_names: Dict[str, List[str]],
    results: List[BenchmarkResult],
):
    for layer in spec_names["layers"]:
        await benchmark(
            "get_layer",
            layer,
            ITER_HEAVY,
            lambda layer_name=layer: statsig_server.get_layer(global_user, layer_name),
            results,
        )


async def main():
    # ------------------------------------------------------------------------ [ Setup ]
    setup_data = setup()
    spec_names = setup_data["spec_names"]
    large_proj_spec_names = setup_data["large_proj_spec_names"]

    options = StatsigOptions(
        api=f"{SCRAPI_URL}/v1",
    )

    statsig.initialize("secret-PYTHON_LEGACY", options)

    large_proj_statsig = StatsigServer()
    large_proj_statsig.initialize("secret-PYTHON_LEGACY::BC_USE_JSON", options)

    results = []

    # ------------------------------------------------------------------------ [ Benchmark ]

    print(f"Statsig Python Legacy (v{sdk_version})")
    print("--------------------------------")

    global_user = StatsigUser(user_id="global_user")

    def init_new_statsig(options: StatsigOptions, sdk_key: str):
        inst = StatsigServer()
        inst.initialize(sdk_key, options)
        return inst

    await benchmark(
        "initialize",
        "json",
        ITER_ULTRA_LITE,
        lambda: init_new_statsig(options, "secret-PYTHON_LEGACY::BC_USE_JSON"),
        results,
        cleanup=lambda inst: inst.shutdown(),
    )

    # ------------------------------------------------------------------------ [ Benchmark Feature Gates ]

    await benchmark_feature_gates(global_user, statsig, spec_names, results)
    await benchmark_feature_gates(
        global_user, large_proj_statsig, large_proj_spec_names, results
    )

    # ------------------------------------------------------------------------ [ Benchmark Dynamic Configs ]

    await benchmark_dynamic_configs(global_user, statsig, spec_names, results)
    await benchmark_dynamic_configs(
        global_user, large_proj_statsig, large_proj_spec_names, results
    )

    config = statsig.get_config(global_user, "operating_system_config")
    await benchmark(
        "get_dynamic_config_params",
        "string",
        ITER_HEAVY,
        lambda: assert_cond(
            config.get_typed("str", "err") != "err", "string value is err"
        ),
        results,
    )

    await benchmark(
        "get_dynamic_config_params",
        "number",
        ITER_HEAVY,
        lambda: assert_cond(config.get_typed("num", 0) != 0, "number value is 0"),
        results,
    )

    await benchmark(
        "get_dynamic_config_params",
        "object",
        ITER_HEAVY,
        lambda: assert_cond(config.get_typed("obj", {}) != {}, "object value is empty"),
        results,
    )

    await benchmark(
        "get_dynamic_config_params",
        "array",
        ITER_HEAVY,
        lambda: assert_cond(config.get_typed("arr", []) != [], "array value is empty"),
        results,
    )

    # ------------------------------------------------------------------------ [ Benchmark Experiments ]

    await benchmark_experiments(global_user, statsig, spec_names, results)
    await benchmark_experiments(
        global_user, large_proj_statsig, large_proj_spec_names, results
    )

    experiment = statsig.get_experiment(global_user, "experiment_with_many_params")
    await benchmark(
        "get_experiment_params",
        "string",
        ITER_HEAVY,
        lambda: assert_cond(
            experiment.get_typed("a_string", "err") != "err", "string value is err"
        ),
        results,
    )

    await benchmark(
        "get_experiment_params",
        "object",
        ITER_HEAVY,
        lambda: assert_cond(
            experiment.get_typed("an_object", {}) != {},
            "object value is empty",
        ),
        results,
    )

    await benchmark(
        "get_experiment_params",
        "array",
        ITER_HEAVY,
        lambda: assert_cond(
            experiment.get_typed("an_array", []) != [], "array value is empty"
        ),
        results,
    )

    # ------------------------------------------------------------------------ [ Benchmark Layers ]

    await benchmark_layers(global_user, statsig, spec_names, results)
    await benchmark_layers(
        global_user, large_proj_statsig, large_proj_spec_names, results
    )

    layer = statsig.get_layer(global_user, "layer_with_many_params")
    await benchmark(
        "get_layer_params",
        "string",
        ITER_HEAVY,
        lambda: assert_cond(
            layer.get_typed("a_string", "err") != "err", "string value is err"
        ),
        results,
    )

    await benchmark(
        "get_layer_params",
        "object",
        ITER_HEAVY,
        lambda: assert_cond(
            layer.get_typed("an_object", {}) != {}, "object value is empty"
        ),
        results,
    )

    await benchmark(
        "get_layer_params",
        "array",
        ITER_HEAVY,
        lambda: assert_cond(
            layer.get_typed("an_array", []) != [], "array value is empty"
        ),
        results,
    )

    # ------------------------------------------------------------------------ [ Benchmark GCIR ]

    await benchmark(
        "get_client_initialize_response",
        "n/a",
        ITER_LITE,
        lambda: statsig.get_client_initialize_response(
            create_user(), hash=HashingAlgorithm.DJB2
        ),
        results,
    )

    await benchmark(
        "get_client_initialize_response_global_user",
        "n/a",
        ITER_LITE,
        lambda: statsig.get_client_initialize_response(
            global_user, hash=HashingAlgorithm.DJB2
        ),
        results,
    )

    await benchmark(
        "user_creation",
        "n/a",
        ITER_HEAVY,
        lambda: create_user_with_benchmark_payload(),
        results,
    )

    # ------------------------------------------------------------------------ [ Teardown ]

    statsig.shutdown()
    large_proj_statsig.shutdown()

    results_file = f"/shared-volume/{sdk_type}-{sdk_version}-results.json"
    with open(results_file, "w") as f:
        json.dump(
            {
                "sdkType": sdk_type,
                "sdkVersion": sdk_version,
                "results": [result.to_dict() for result in results],
            },
            f,
            indent=2,
        )


if __name__ == "__main__":
    asyncio.run(main())
