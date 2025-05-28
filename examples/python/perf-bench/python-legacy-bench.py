import os
import time
import numpy as np
import random
from statsig import statsig, StatsigUser, StatsigEvent

from importlib.metadata import version

sdk_version = version("statsig")

statsig.initialize(os.getenv("PERF_SDK_KEY"))

iterations = 100_000
global_user = StatsigUser(user_id="global_user")

results = {}


def log_benchmark(name, p99):
    print(f"{name.ljust(50)} {p99:.4f}ms")
    event = StatsigEvent(
        user=global_user,
        event_name="sdk_benchmark",
        value=p99,
        metadata={
            "benchmarkName": name,
            "sdkType": "py-server",
            "sdkVersion": sdk_version,
        },
    )
    statsig.log_event(event)


def make_random_user():
    return StatsigUser(user_id=f"user_{random.randint(0, 1000000)}")


def benchmark(func):
    durations = []

    for _ in range(iterations):
        start = time.perf_counter()
        func()
        end = time.perf_counter()
        durations.append((end - start) * 1000)  # ms

    return np.percentile(durations, 99)


def run_check_gate():
    def action():
        user = make_random_user()
        statsig.check_gate(user, "test_public")

    p99 = benchmark(action)
    results["check_gate"] = p99


def run_check_gate_global_user():
    def action():
        statsig.check_gate(global_user, "test_public")

    p99 = benchmark(action)
    results["check_gate_global_user"] = p99


def run_get_feature_gate():
    def action():
        user = make_random_user()
        statsig.get_feature_gate(user, "test_public")

    p99 = benchmark(action)
    results["get_feature_gate"] = p99


def run_get_feature_gate_global_user():
    def action():
        statsig.get_feature_gate(global_user, "test_public")

    p99 = benchmark(action)
    results["get_feature_gate_global_user"] = p99


def run_get_experiment():
    def action():
        user = make_random_user()
        statsig.get_experiment(user, "an_experiment")

    p99 = benchmark(action)
    results["get_experiment"] = p99


def run_get_experiment_global_user():
    def action():
        statsig.get_experiment(global_user, "an_experiment")

    p99 = benchmark(action)
    results["get_experiment_global_user"] = p99


def run_get_client_initialize_response():
    def action():
        statsig.get_client_initialize_response(global_user)

    p99 = benchmark(action)
    results["get_client_initialize_response"] = p99


def run_get_client_initialize_response_global_user():
    def action():
        statsig.get_client_initialize_response(global_user)

    p99 = benchmark(action)
    results["get_client_initialize_response_global_user"] = p99


if __name__ == "__main__":
    print(f"Statsig Python Legacy (v{sdk_version})")
    print("--------------------------------")
    run_check_gate()
    run_check_gate_global_user()
    run_get_feature_gate()
    run_get_feature_gate_global_user()
    run_get_experiment()
    run_get_experiment_global_user()
    run_get_client_initialize_response()
    run_get_client_initialize_response_global_user()

    for name, p99 in results.items():
        log_benchmark(name, p99)

    statsig.shutdown()
    print("\n\n")
