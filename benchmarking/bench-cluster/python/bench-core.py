import asyncio
import random
import string
import time
from typing import List, Dict, Any, Callable, Optional
import os
from statsig_python_core import (
    Statsig,
    StatsigUser,
    StatsigOptions,
)
from importlib.metadata import version
import json

sdk_type = "statsig-server-core-python"
sdk_version = version("statsig-python-core")

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


def setup():
    # Wait for spec_names.json to be available
    spec_name_path = "/shared-volume/spec_names.json"
    for i in range(10):
        if os.path.exists(spec_name_path):
            break
        time.sleep(1)

    with open(spec_name_path, "r") as f:
        return json.load(f)


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


async def main():
    # ------------------------------------------------------------------------ [ Setup ]
    spec_names = setup()

    options = StatsigOptions(
        specs_url=f"{SCRAPI_URL}/v2/download_config_specs",
        log_event_url=f"{SCRAPI_URL}/v1/log_event",
    )

    statsig = Statsig("secret-PYTHON_CORE", options)
    statsig.initialize().wait()

    results = []

    # ------------------------------------------------------------------------ [ Benchmark ]

    print(f"Statsig Python Core (v{sdk_version})")
    print("--------------------------------")

    global_user = StatsigUser(user_id=f"global_user")

    def init_new_statsig(options: StatsigOptions, sdk_key: str):
        inst = Statsig(sdk_key, options)
        inst.initialize().wait()
        return inst

    await benchmark(
        "initialize",
        "proto",
        ITER_ULTRA_LITE,
        lambda: init_new_statsig(options, "secret-PYTHON_CORE::BC_USE_PROTO"),
        results,
        cleanup=lambda inst: inst.shutdown().wait(),
    )

    await benchmark(
        "initialize",
        "json",
        ITER_ULTRA_LITE,
        lambda: init_new_statsig(options, "secret-PYTHON_CORE::BC_USE_JSON"),
        results,
        cleanup=lambda inst: inst.shutdown().wait(),
    )

    # ------------------------------------------------------------------------ [ Benchmark Feature Gates ]

    for gate in spec_names["feature_gates"]:
        await benchmark(
            "check_gate",
            gate,
            ITER_HEAVY,
            lambda g=gate: statsig.check_gate(create_user(), g),
            results,
        )

        await benchmark(
            "check_gate_global_user",
            gate,
            ITER_HEAVY,
            lambda g=gate: statsig.check_gate(global_user, g),
            results,
        )

        await benchmark(
            "get_feature_gate",
            gate,
            ITER_HEAVY,
            lambda g=gate: statsig.get_feature_gate(create_user(), g),
            results,
        )

        await benchmark(
            "get_feature_gate_global_user",
            gate,
            ITER_HEAVY,
            lambda g=gate: statsig.get_feature_gate(global_user, g),
            results,
        )

    # ------------------------------------------------------------------------ [ Benchmark Dynamic Configs ]

    for config in spec_names["dynamic_configs"]:
        await benchmark(
            "get_dynamic_config",
            config,
            ITER_HEAVY,
            lambda c=config: statsig.get_dynamic_config(create_user(), c),
            results,
        )

        await benchmark(
            "get_dynamic_config_global_user",
            config,
            ITER_HEAVY,
            lambda c=config: statsig.get_dynamic_config(global_user, c),
            results,
        )

    config = statsig.get_dynamic_config(global_user, "operating_system_config")
    await benchmark(
        "get_dynamic_config_params",
        "string",
        ITER_HEAVY,
        lambda: assert_cond(
            config.get_string("str", "err") != "err", "string value is err"
        ),
        results,
    )
    await benchmark(
        "get_dynamic_config_params",
        "number",
        ITER_HEAVY,
        lambda: assert_cond(config.get_integer("num", 0) != 0, "number value is 0"),
        results,
    )
    await benchmark(
        "get_dynamic_config_params",
        "object",
        ITER_HEAVY,
        lambda: assert_cond(
            (
                config.get_object_json("obj", "") != ""
                if hasattr(config, "get_object_json")
                else config.get_object("obj", {}) != {}
            ),
            "object value is empty",
        ),
        results,
    )
    await benchmark(
        "get_dynamic_config_params",
        "array",
        ITER_HEAVY,
        lambda: assert_cond(
            (
                config.get_array_json("arr", "") != ""
                if hasattr(config, "get_array_json")
                else config.get_array("arr", []) != []
            ),
            "array value is empty",
        ),
        results,
    )

    # ------------------------------------------------------------------------ [ Benchmark Experiments ]

    for experiment in spec_names["experiments"]:
        await benchmark(
            "get_experiment",
            experiment,
            ITER_HEAVY,
            lambda e=experiment: statsig.get_experiment(create_user(), e),
            results,
        )

        await benchmark(
            "get_experiment_global_user",
            experiment,
            ITER_HEAVY,
            lambda e=experiment: statsig.get_experiment(global_user, e),
            results,
        )

    experiment = statsig.get_experiment(global_user, "experiment_with_many_params")
    await benchmark(
        "get_experiment_params",
        "string",
        ITER_HEAVY,
        lambda: assert_cond(
            experiment.get_string("a_string", "err") != "err", "string value is err"
        ),
        results,
    )
    await benchmark(
        "get_experiment_params",
        "object",
        ITER_HEAVY,
        lambda: assert_cond(
            (
                experiment.get_object_json("an_object", "") != ""
                if hasattr(experiment, "get_object_json")
                else experiment.get_object("an_object", {}) != {}
            ),
            "object value is empty",
        ),
        results,
    )
    await benchmark(
        "get_experiment_params",
        "array",
        ITER_HEAVY,
        lambda: assert_cond(
            (
                experiment.get_array_json("an_array", "") != ""
                if hasattr(experiment, "get_array_json")
                else experiment.get_array("an_array", []) != []
            ),
            "array value is empty",
        ),
        results,
    )

    # ------------------------------------------------------------------------ [ Benchmark Layers ]

    for layer in spec_names["layers"]:
        await benchmark(
            "get_layer",
            layer,
            ITER_HEAVY,
            lambda l=layer: statsig.get_layer(create_user(), l),
            results,
        )

        await benchmark(
            "get_layer_global_user",
            layer,
            ITER_HEAVY,
            lambda l=layer: statsig.get_layer(global_user, l),
            results,
        )

    layer = statsig.get_layer(global_user, "layer_with_many_params")
    await benchmark(
        "get_layer_params",
        "string",
        ITER_HEAVY,
        lambda: assert_cond(
            layer.get_string("a_string", "err") != "err", "string value is err"
        ),
        results,
    )
    await benchmark(
        "get_layer_params",
        "object",
        ITER_HEAVY,
        lambda: assert_cond(
            (
                layer.get_object_json("an_object", "") != ""
                if hasattr(layer, "get_object_json")
                else layer.get_object("an_object", {}) != {}
            ),
            "object value is empty",
        ),
        results,
    )
    await benchmark(
        "get_layer_params",
        "array",
        ITER_HEAVY,
        lambda: assert_cond(
            (
                layer.get_array_json("an_array", "") != ""
                if hasattr(layer, "get_array_json")
                else layer.get_array("an_array", []) != []
            ),
            "array value is empty",
        ),
        results,
    )

    # ------------------------------------------------------------------------ [ Benchmark GCIR ]

    await benchmark(
        "get_client_initialize_response",
        "n/a",
        ITER_LITE,
        lambda: statsig.get_client_initialize_response(create_user()),
        results,
    )

    await benchmark(
        "get_client_initialize_response_global_user",
        "n/a",
        ITER_LITE,
        lambda: statsig.get_client_initialize_response(global_user),
        results,
    )

    # ------------------------------------------------------------------------ [ Teardown ]

    statsig.shutdown().wait()

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
