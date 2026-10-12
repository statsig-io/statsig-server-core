import json
import os
import shutil
import socket
import subprocess
import sys
import time
from contextlib import contextmanager
from pathlib import Path

import pytest

TESTS_DIR = Path(__file__).parents[2]
sys.path.insert(0, str(TESTS_DIR))

from mock_scrapi import MockScrapi
from pytest_httpserver import HTTPServer
from utils import get_test_data_resource


pytestmark = [
    pytest.mark.skipif(
        not sys.platform.startswith("linux"),
        reason="Celery prefork production behavior is validated on Linux",
    ),
    pytest.mark.skipif(
        os.environ.get("STATSIG_RUN_CELERY_PREFORK_TESTS") != "1",
        reason="set STATSIG_RUN_CELERY_PREFORK_TESTS=1 to run Celery integration tests",
    ),
]

Celery = pytest.importorskip("celery").Celery


SDK_KEY = "secret-celery-prefork"
DCS_ENDPOINT = f"/v2/download_config_specs/{SDK_KEY}.json"
TASK_NAME = "statsig_tests.evaluate_and_log"
INTEGRATION_DIR = Path(__file__).parent


@pytest.mark.parametrize(
    "max_tasks_per_child",
    [None, 1],
    ids=["stable-workers", "recycled-workers"],
)
def test_initialized_master_survives_celery_prefork(
    httpserver: HTTPServer,
    tmp_path: Path,
    max_tasks_per_child: int | None,
):
    mock_scrapi, dcs_state = _setup_server(httpserver)
    task_count = 6

    with _redis_broker(tmp_path) as broker_url:
        environment = {
            **os.environ,
            "CELERY_BROKER_URL": broker_url,
            "STATSIG_TEST_SPECS_URL": mock_scrapi.url_for_endpoint(
                "/v2/download_config_specs"
            ),
            "STATSIG_TEST_LOG_EVENT_URL": mock_scrapi.url_for_endpoint(
                "/v1/log_event"
            ),
            "STATSIG_TEST_ID_LISTS_URL": mock_scrapi.url_for_endpoint(
                "/v1/get_id_lists"
            ),
            "STATSIG_TEST_OVERRIDE_TICK_INTERVAL_MS": "100",
            "STATSIG_TEST_OVERRIDE_MIN_FLUSH_INTERVAL_MS": "100",
            "STATSIG_TEST_OVERRIDE_MAX_FLUSH_INTERVAL_MS": "100",
        }

        with _celery_worker(
            broker_url,
            environment,
            tmp_path,
            max_tasks_per_child=max_tasks_per_child,
        ) as client:
            first_results = [
                client.send_task(TASK_NAME, args=[task_index, False])
                for task_index in range(3)
            ]
            _wait_for_active_tasks(client, 3)
            dcs_state["response"] = _dcs_with_gate_value(False, 1)
            first_values = [result.get(timeout=30) for result in first_results]

            # Let the master and replacement workers settle on the first
            # update. The second wave starts while false is still served, so
            # every task must already be running before the response flips.
            time.sleep(1.25)
            second_results = [
                client.send_task(TASK_NAME, args=[task_index, True])
                for task_index in range(3, task_count)
            ]
            _wait_for_active_tasks(client, 3)
            dcs_state["response"] = _dcs_with_gate_value(True, 2)
            second_values = [result.get(timeout=30) for result in second_results]
            values = first_values + second_values

            assert [value["task_index"] for value in values] == list(
                range(task_count)
            )
            assert [value["gate_value"] for value in first_values] == [False] * 3
            assert [value["gate_value"] for value in second_values] == [True] * 3
            assert all(value["worker_init_started"] for value in values)
            assert all(value["worker_init_completed"] for value in values)
            assert not any(value["task_started_before_recovery"] for value in values)
            for value in values:
                assert len(value["recovery_calls"]) == 1
                recovery_call = value["recovery_calls"][0]
                assert recovery_call["pid"] == value["worker_pid"]
                assert recovery_call["phase"] == "worker_process_init"
                assert recovery_call["success"] is True
                assert 0 < recovery_call["timeout_ms"] <= 3000

            if max_tasks_per_child == 1:
                assert len({value["worker_pid"] for value in values}) == task_count
            else:
                first_worker_pids = {value["worker_pid"] for value in first_values}
                second_worker_pids = {
                    value["worker_pid"] for value in second_values
                }
                assert len(first_worker_pids) == 3
                assert first_worker_pids == second_worker_pids

            _wait_for_logged_task_values(mock_scrapi, values)


def _setup_server(httpserver: HTTPServer) -> tuple[MockScrapi, dict[str, str]]:
    mock_scrapi = MockScrapi(httpserver)
    dcs_state = {"response": _dcs_with_gate_value(True, 0)}
    mock_scrapi.stub(
        DCS_ENDPOINT,
        response=lambda _request: dcs_state["response"],
        method="GET",
    )
    mock_scrapi.stub("/v1/log_event", response='{"success": true}', method="POST")
    mock_scrapi.stub("/v1/get_id_lists", response="{}", method="POST")
    return mock_scrapi, dcs_state


def _dcs_with_gate_value(value: bool, revision: int) -> str:
    dcs = json.loads(get_test_data_resource("eval_proj_dcs.json"))
    dcs["time"] += revision
    dcs["checksum"] = f"celery-prefork-revision-{revision}"
    gate = dcs["feature_gates"]["test_public"]
    gate["enabled"] = True
    gate["defaultValue"] = value
    gate["rules"] = []
    gate["version"] += revision
    return json.dumps(dcs)


@contextmanager
def _redis_broker(tmp_path: Path):
    configured_url = os.environ.get("STATSIG_CELERY_BROKER_URL")
    if configured_url:
        yield configured_url
        return

    redis_server = shutil.which("redis-server")
    if redis_server is None:
        pytest.skip("redis-server is required for the Celery prefork integration test")

    port = _unused_local_port()
    process = subprocess.Popen(
        [
            redis_server,
            "--port",
            str(port),
            "--save",
            "",
            "--appendonly",
            "no",
            "--dir",
            str(tmp_path),
        ],
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )
    try:
        _wait_for_port(port, process)
        yield f"redis://127.0.0.1:{port}/0"
    finally:
        process.terminate()
        try:
            process.wait(timeout=5)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait(timeout=5)


@contextmanager
def _celery_worker(
    broker_url: str,
    environment: dict[str, str],
    tmp_path: Path,
    max_tasks_per_child: int | None,
):
    command = [
        sys.executable,
        "-m",
        "celery",
        "-A",
        "celery_app:app",
        "worker",
        "--pool=prefork",
        "--concurrency=3",
        "--without-gossip",
        "--without-mingle",
        "--without-heartbeat",
        "--loglevel=INFO",
    ]
    if max_tasks_per_child is not None:
        command.append(f"--max-tasks-per-child={max_tasks_per_child}")

    log_path = tmp_path / "celery-worker.log"
    with log_path.open("w+") as worker_log:
        process = subprocess.Popen(
            command,
            cwd=INTEGRATION_DIR,
            env=environment,
            stdout=worker_log,
            stderr=subprocess.STDOUT,
        )
        client = Celery("statsig-prefork-driver", broker=broker_url, backend=broker_url)
        try:
            _wait_for_worker(client, process, worker_log)
            yield client
        finally:
            result_consumer = client.backend.result_consumer
            result_consumer.stop()
            # Celery leaves the closed pubsub attached, so late AsyncResult
            # finalizers otherwise reconnect after the temporary broker stops.
            result_consumer._pubsub = None
            client.close()
            process.terminate()
            try:
                process.wait(timeout=10)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait(timeout=5)


def _wait_for_worker(client: Celery, process, worker_log, timeout_seconds: float = 20):
    deadline = time.monotonic() + timeout_seconds
    while time.monotonic() < deadline:
        if process.poll() is not None:
            worker_log.flush()
            worker_log.seek(0)
            raise AssertionError(
                f"Celery worker exited with {process.returncode}:\n{worker_log.read()}"
            )
        if client.control.ping(timeout=1):
            return
        time.sleep(0.25)

    worker_log.flush()
    worker_log.seek(0)
    raise AssertionError(f"Celery worker did not become ready:\n{worker_log.read()}")


def _wait_for_active_tasks(client: Celery, expected: int, timeout_seconds: float = 20):
    deadline = time.monotonic() + timeout_seconds
    while time.monotonic() < deadline:
        active = client.control.inspect(timeout=1).active() or {}
        count = sum(
            task.get("name") == TASK_NAME
            for tasks in active.values()
            for task in tasks
        )
        if count == expected:
            return
        time.sleep(0.1)
    raise AssertionError(f"expected {expected} active Statsig tasks, observed {count}")


def _wait_for_logged_task_values(
    mock_scrapi: MockScrapi,
    task_results: list[dict[str, object]],
) -> None:
    expected_values = {
        f'{result["task_index"]}:{result["worker_pid"]}' for result in task_results
    }
    deadline = time.monotonic() + 10
    while time.monotonic() < deadline:
        observed_values = [
            str(event.get("value"))
            for event in mock_scrapi.get_logged_events()
            if event.get("eventName") == "celery-prefork-event"
        ]
        if set(observed_values) == expected_values and len(observed_values) == len(
            expected_values
        ):
            return
        time.sleep(0.1)

    raise AssertionError(
        "Celery tasks completed but automatic event flushing did not deliver "
        "exactly one event per worker task; "
        f"observed {observed_values}, expected {sorted(expected_values)}"
    )


def _unused_local_port() -> int:
    with socket.socket() as sock:
        sock.bind(("127.0.0.1", 0))
        return sock.getsockname()[1]


def _wait_for_port(port: int, process, timeout_seconds: float = 5) -> None:
    deadline = time.monotonic() + timeout_seconds
    while time.monotonic() < deadline:
        if process.poll() is not None:
            raise AssertionError(f"redis-server exited with {process.returncode}")
        with socket.socket() as sock:
            sock.settimeout(0.2)
            if sock.connect_ex(("127.0.0.1", port)) == 0:
                return
        time.sleep(0.05)
    raise AssertionError(f"redis-server did not listen on port {port}")
