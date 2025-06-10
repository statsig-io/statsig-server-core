import subprocess
import psutil
import time
import numpy as np
import argparse
import shlex
import requests
import os
import tempfile
import json


def post_metrics(sdk_type, sdk_version, cpu_p99, mem_p99_bytes):
    events = [
        {
            "eventName": "sdk_benchmark",
            "value": cpu_p99,
            "user": {"userID": "gh_profiler"},
            "time": round(time.time() * 1000),
            "metadata": {
                "benchmarkName": "overall_cpu_usage",
                "sdkType": sdk_type,
                "sdkVersion": sdk_version,
            },
        },
        {
            "eventName": "sdk_benchmark",
            "value": mem_p99_bytes,
            "user": {"userID": "gh_profiler"},
            "time": round(time.time() * 1000),
            "metadata": {
                "benchmarkName": "overall_mem_usage",
                "sdkType": sdk_type,
                "sdkVersion": sdk_version,
            },
        },
    ]

    result = requests.post(
        "https://events.statsigapi.net/v1/log_event",
        json={"events": events},
        headers={
            "STATSIG-API-KEY": os.getenv("PERF_SDK_KEY"),
        },
    )

    if result.status_code >= 200 and result.status_code < 300:
        print("Successfully sent overall usage metrics")
    else:
        raise Exception(
            f"Failed to send overall usage metrics: {result.status_code} {result.text}"
        )


def read_result_file(tmp_path):
    with open(tmp_path, "r") as f:
        try:
            result_data = json.load(f)
            return result_data
        except json.JSONDecodeError:
            raise Exception("Subprocess did not write valid JSON")


def run_and_monitor(command, interval=0.5):
    with tempfile.NamedTemporaryFile(delete=False) as tmpfile:
        tmp_path = tmpfile.name

    env = os.environ.copy()
    env["BENCH_METADATA_FILE"] = tmp_path

    # Start subprocess
    proc = subprocess.Popen(command, env=env)
    ps_proc = psutil.Process(proc.pid)

    cpu_samples = []
    mem_samples = []

    try:
        while proc.poll() is None:
            try:
                cpu = ps_proc.cpu_percent(interval=None)
                mem = ps_proc.memory_info().rss / (1024**2)  # Convert to MB
            except psutil.NoSuchProcess:
                break

            cpu_samples.append(cpu)
            mem_samples.append(mem)

            time.sleep(interval)
    finally:
        if proc.poll() is None:
            proc.terminate()

    cpu_p99 = np.percentile(cpu_samples, 99)
    mem_p99 = np.percentile(mem_samples, 99)
    mem_p99_bytes = mem_p99 * 1024 * 1024

    bench_metadata = read_result_file(tmp_path)

    print(f"SDK Type: {bench_metadata['sdk_type']}")
    print(f"SDK Version: {bench_metadata['sdk_version']}")
    print(f"P99 CPU usage: {cpu_p99:.2f}%")
    print(f"P99 Memory usage: {mem_p99:.2f} MB")

    post_metrics(
        bench_metadata["sdk_type"],
        bench_metadata["sdk_version"],
        cpu_p99,
        mem_p99_bytes,
    )


if __name__ == "__main__":
    parser = argparse.ArgumentParser(
        description="Monitor CPU and memory usage of a subprocess."
    )
    parser.add_argument(
        "command", nargs=argparse.REMAINDER, help="Command to run and monitor."
    )
    args = parser.parse_args()

    if not args.command:
        print("Error: No command provided.")
        exit(1)

    run_and_monitor(args.command)
