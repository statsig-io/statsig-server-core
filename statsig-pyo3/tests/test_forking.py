import asyncio
import datetime
import json
import multiprocessing
import os
import subprocess
import sys
import threading
import time
from pathlib import Path
from typing import Generator

import pytest
import uvloop
from mock_scrapi import MockScrapi
from pytest_httpserver import HTTPServer
from statsig_python_core import (
    DataStore,
    DataStoreResponse,
    Statsig,
    StatsigOptions,
    StatsigUser,
    handle_fork,
    handle_after_fork_in_child,
    recover_after_fork,
)
from utils import get_test_data_resource

multiprocessing.set_start_method("fork", force=True)

DCS_URL_ENDPOINT = "/v2/download_config_specs"
SYNC_TEST_SDK_KEY = "secret-forking-sync-test"
SYNC_TEST_DCS_ENDPOINT = f"{DCS_URL_ENDPOINT}/{SYNC_TEST_SDK_KEY}.json"
SPECS_SYNC_INTERVAL_MS = 1000
SPECS_SYNC_TIMEOUT_SECONDS = 4
EAGER_POSTFORK_PROCESS_COUNT = 1 if sys.platform == "darwin" else 2


class PollingForkDataStore(DataStore):
    def __init__(self):
        super().__init__()
        self.initialize_fn = self.initialize
        self.shutdown_fn = self.shutdown
        self.get_fn = self.get
        self.set_fn = self.set
        self.support_polling_updates_for_fn = self.support_polling_updates_for

    def initialize(self):
        pass

    def shutdown(self):
        pass

    def get(self, _key):
        return DataStoreResponse(
            result=get_test_data_resource("eval_proj_dcs.json"), time=1
        )

    def set(self, _key, _value, _time=None):
        pass

    def support_polling_updates_for(self, _key):
        return True


def setup_server(httpserver: HTTPServer):
    mock_scrapi = MockScrapi(httpserver)
    mock_scrapi.stub("/v1/log_event", response='{"success": true}', method="POST")
    dcs_content = get_test_data_resource("eval_proj_dcs.json")
    mock_scrapi.stub(
        "/v2/download_config_specs/secret-forking-test.json",
        response=dcs_content,
        method="GET",
    )
    mock_scrapi.stub(
        SYNC_TEST_DCS_ENDPOINT,
        response=dcs_content,
        method="GET",
    )
    mock_scrapi.stub("/v1/get_id_lists", response="{}", method="POST")

    return mock_scrapi


def test_forking(httpserver: HTTPServer):
    mock_scrapi = setup_server(httpserver)

    specs_url = mock_scrapi.url_for_endpoint("/v2/download_config_specs")
    log_event_url = mock_scrapi.url_for_endpoint("/v1/log_event")
    id_lists_url = mock_scrapi.url_for_endpoint("/v1/get_id_lists")

    command = [
        sys.executable,
        str(Path(__file__).with_name("fork_runner.py")),
        specs_url,
        log_event_url,
        id_lists_url,
    ]
    print("Running command: ", command)
    proc = subprocess.Popen(
        command,
        universal_newlines=True,
        env={**os.environ, "RUST_BACKTRACE": "full"},
    )
    try:
        proc.communicate(timeout=10)
    except subprocess.TimeoutExpired:
        proc.terminate()
        proc.wait()

    assert proc.returncode == 0


def test_background_specs_sync_recovers_after_inline_fork(httpserver: HTTPServer):
    mock_scrapi = setup_server(httpserver)
    _assert_background_specs_sync_recovers_after_fork(mock_scrapi)


def test_polling_data_store_recovery_releases_gil_for_all_live_instances(httpserver: HTTPServer):
    # Polling callbacks reacquire the GIL while recovery waits for scheduling.
    stores = [PollingForkDataStore(), PollingForkDataStore()]
    instances = [
        Statsig(
            f"secret-polling-fork-{index}",
            StatsigOptions(
                specs_url=httpserver.url_for(DCS_URL_ENDPOINT),
                data_store=store,
                disable_network=True,
            ),
        )
        for index, store in enumerate(stores)
    ]
    try:
        for instance in instances:
            instance.initialize().wait()

        handle_fork()
        started = time.monotonic()
        assert recover_after_fork()
        assert time.monotonic() - started < 1, "post-fork recovery held the GIL"
    finally:
        for instance in instances:
            instance.shutdown().wait()


def test_raw_child_hook_only_marks_deferred_recovery(monkeypatch):
    import statsig_python_core.statsig as statsig_module

    class ForkHookProbe:
        def __init__(self):
            self.call_count = 0
            self.schedule_count = 0
            self.recovered = False

        def _INTERNAL_restart_background_tasks_after_fork(
            self, _timeout_ms: int
        ) -> bool:
            self.call_count += 1
            if not self.recovered:
                self.schedule_count += 1
                self.recovered = True
            return True

    probe = ForkHookProbe()
    monkeypatch.setattr(statsig_module, "_live_statsig_instances", {probe})
    monkeypatch.setattr(statsig_module, "_child_recovery_pending", False)

    threads_before = threading.active_count()
    handle_after_fork_in_child()

    assert statsig_module._child_recovery_pending
    assert probe.call_count == 0
    assert probe.schedule_count == 0
    assert threading.active_count() == threads_before

    assert recover_after_fork()
    assert not statsig_module._child_recovery_pending
    assert probe.call_count == 1
    assert probe.schedule_count == 1

    assert recover_after_fork()
    assert probe.call_count == 2
    assert probe.schedule_count == 1


def test_failed_deferred_child_recovery_remains_pending(monkeypatch):
    import statsig_python_core.statsig as statsig_module

    class ForkHookProbe:
        def __init__(self):
            self.restart_results = iter([False, True])
            self.restart_count = 0

        def _INTERNAL_restart_background_tasks_after_fork(
            self, _timeout_ms: int
        ) -> bool:
            self.restart_count += 1
            return next(self.restart_results)

    probe = ForkHookProbe()
    monkeypatch.setattr(statsig_module, "_live_statsig_instances", {probe})
    monkeypatch.setattr(statsig_module, "_child_recovery_pending", True)

    assert not recover_after_fork()
    assert statsig_module._child_recovery_pending
    assert recover_after_fork()
    assert not statsig_module._child_recovery_pending
    assert probe.restart_count == 2


def test_failed_instances_do_not_block_later_live_instance_recovery(monkeypatch):
    import statsig_python_core.statsig as statsig_module

    calls: list[str] = []

    class Probe:
        def __init__(self, name: str, result: bool | Exception):
            self.name = name
            self.result = result

        def _INTERNAL_restart_background_tasks_after_fork(
            self, _timeout_ms: int
        ) -> bool:
            calls.append(self.name)
            if isinstance(self.result, Exception):
                raise self.result
            return self.result

    probes = [
        Probe("failed", False),
        Probe("raised", RuntimeError("recovery failed")),
        Probe("later-live", True),
    ]
    monkeypatch.setattr(statsig_module, "_live_statsig_instances", probes)
    monkeypatch.setattr(statsig_module, "_child_recovery_pending", True)

    assert not recover_after_fork()
    assert calls == ["failed", "raised", "later-live"]
    assert statsig_module._child_recovery_pending


def test_recovery_uses_one_callback_wide_deadline(monkeypatch):
    import statsig_python_core.statsig as statsig_module

    class SlowProbe:
        def __init__(self):
            self.timeouts: list[int] = []

        def _INTERNAL_restart_background_tasks_after_fork(
            self, timeout_ms: int
        ) -> bool:
            self.timeouts.append(timeout_ms)
            time.sleep((timeout_ms + 10) / 1000)
            return False

    first = SlowProbe()
    skipped = SlowProbe()
    monkeypatch.setattr(statsig_module, "_live_statsig_instances", [first, skipped])
    monkeypatch.setattr(statsig_module, "POST_FORK_RECOVERY_BUDGET_SECONDS", 0.05)
    monkeypatch.setattr(statsig_module, "_child_recovery_pending", True)

    started = time.monotonic()
    assert not recover_after_fork()
    elapsed = time.monotonic() - started

    assert first.timeouts and first.timeouts[0] <= 50
    assert skipped.timeouts == []
    assert elapsed < 0.2
    assert statsig_module._child_recovery_pending


@pytest.mark.skipif(
    sys.platform == "darwin",
    reason="thread creation in a multi-threaded fork child traps on macOS",
)
def test_deferred_postfork_sync_is_nonblocking_and_refreshes_parent_and_child(
    httpserver: HTTPServer,
):
    mock_scrapi = MockScrapi(httpserver)
    mock_scrapi.stub("/v1/log_event", response='{"success": true}', method="POST")
    mock_scrapi.stub("/v1/get_id_lists", response="{}", method="POST")

    original = json.loads(get_test_data_resource("eval_proj_dcs.json"))
    updated = json.loads(json.dumps(original))
    updated["time"] = original["time"] + 1
    updated["checksum"] = "post-fork-refresh"
    updated_gate = updated["feature_gates"]["test_public"]
    updated_gate["enabled"] = False
    updated_gate["rules"] = []
    updated_gate["version"] += 1

    block_postfork_responses = threading.Event()
    release_postfork_responses = threading.Event()

    def specs_response(_request):
        if block_postfork_responses.is_set():
            release_postfork_responses.wait(timeout=5)
            return json.dumps(updated)
        return json.dumps(original)

    mock_scrapi.stub(SYNC_TEST_DCS_ENDPOINT, response=specs_response, method="GET")

    statsig = _new_statsig(mock_scrapi, specs_sync_interval_ms=60_000)
    user = StatsigUser("fork-refresh-user")
    allow_child_evaluation = multiprocessing.Event()
    child_result, child_sender = multiprocessing.Pipe(duplex=False)
    process = multiprocessing.Process(
        target=_evaluate_while_postfork_refresh_is_blocked,
        args=(statsig, user, allow_child_evaluation, child_sender),
    )

    try:
        statsig.initialize().wait()
        assert statsig.check_gate(user, "test_public")

        requests_before_fork = mock_scrapi.times_called_for_endpoint(
            SYNC_TEST_DCS_ENDPOINT
        )
        block_postfork_responses.set()

        fork_started = time.monotonic()
        process.start()
        assert time.monotonic() - fork_started < 2

        # At least one automatic post-fork refresh reaches the deliberately
        # blocked single-threaded server. The fork itself remains nonblocking.
        _wait_for_endpoint_calls(
            mock_scrapi,
            SYNC_TEST_DCS_ENDPOINT,
            requests_before_fork + 1,
        )

        allow_child_evaluation.set()
        assert child_result.poll(2), "child did not reach its first evaluation"
        child_value, child_evaluation_seconds = child_result.recv()
        assert child_value is True
        assert child_evaluation_seconds < 1

        parent_evaluation_started = time.monotonic()
        assert statsig.check_gate(user, "test_public")
        assert time.monotonic() - parent_evaluation_started < 1

        release_postfork_responses.set()

        # The mock server serializes request handlers, so the second process's
        # request cannot enter the handler until the first response is released.
        # Both requests completing proves parent and child independently began
        # their automatic refresh without an SDK evaluation triggering either.
        _wait_for_endpoint_calls(
            mock_scrapi,
            SYNC_TEST_DCS_ENDPOINT,
            requests_before_fork + EAGER_POSTFORK_PROCESS_COUNT,
        )

        assert _wait_for_gate_value(statsig, user, False)
        assert child_result.poll(3), "child did not apply refreshed config"
        assert child_result.recv() is False

        process.join(timeout=5)
        assert process.exitcode == 0
    finally:
        release_postfork_responses.set()
        allow_child_evaluation.set()
        if process.is_alive():
            process.terminate()
            process.join(timeout=5)
        statsig.shutdown().wait()


@pytest.mark.skipif(
    sys.platform == "darwin",
    reason="thread creation in a multi-threaded fork child traps on macOS",
)
def test_public_recovery_handles_multiple_live_and_shutdown_instances(
    httpserver: HTTPServer,
):
    mock_scrapi = MockScrapi(httpserver)
    dcs_content = get_test_data_resource("eval_proj_dcs.json")
    sdk_keys = [
        "secret-fork-instance-a",
        "secret-fork-instance-b",
        "secret-fork-instance-shutdown",
    ]
    endpoints = [f"{DCS_URL_ENDPOINT}/{key}.json" for key in sdk_keys]
    for endpoint in endpoints:
        mock_scrapi.stub(endpoint, response=dcs_content, method="GET")
    mock_scrapi.stub("/v1/log_event", response='{"success": true}', method="POST")
    mock_scrapi.stub("/v1/get_id_lists", response="{}", method="POST")

    instances = [
        _new_statsig(mock_scrapi, sdk_key=key, specs_sync_interval_ms=60_000)
        for key in sdk_keys
    ]
    process = None

    try:
        for instance in instances:
            instance.initialize().wait()
        instances[-1].shutdown().wait()
        baselines = [
            mock_scrapi.times_called_for_endpoint(endpoint) for endpoint in endpoints
        ]

        process = multiprocessing.Process(target=_recover_and_sleep_task)
        process.start()
        process.join(timeout=5)
        assert process.exitcode == 0

        for endpoint, baseline in zip(endpoints[:-1], baselines[:-1]):
            _wait_for_endpoint_calls(
                mock_scrapi,
                endpoint,
                baseline + EAGER_POSTFORK_PROCESS_COUNT,
            )
        time.sleep(0.2)
        assert (
            mock_scrapi.times_called_for_endpoint(endpoints[-1]) == baselines[-1]
        )
    finally:
        if process is not None and process.is_alive():
            process.terminate()
            process.join(timeout=5)
        for instance in instances:
            instance.shutdown().wait()


def test_shutdown_instance_is_not_restarted_after_fork(httpserver: HTTPServer):
    mock_scrapi = setup_server(httpserver)
    statsig = _new_statsig(mock_scrapi)
    process = None

    statsig.initialize().wait()
    _wait_for_next_specs_sync(mock_scrapi)
    statsig.shutdown().wait()
    calls_after_shutdown = mock_scrapi.times_called_for_endpoint(SYNC_TEST_DCS_ENDPOINT)

    try:
        process = multiprocessing.Process(target=_pass_through_task)
        process.start()
        process.join(timeout=5)
        assert process.exitcode == 0

        time.sleep((SPECS_SYNC_INTERVAL_MS / 1000) * 1.5)
        assert (
            mock_scrapi.times_called_for_endpoint(SYNC_TEST_DCS_ENDPOINT)
            == calls_after_shutdown
        )
    finally:
        if process is not None and process.is_alive():
            process.terminate()
            process.join(timeout=5)


@pytest.mark.skipif(
    sys.platform == "darwin",
    reason="thread creation in a multi-threaded fork child traps on macOS",
)
def test_repeated_forks_keep_parent_sync_alive(httpserver: HTTPServer):
    mock_scrapi = setup_server(httpserver)
    statsig = _new_statsig(mock_scrapi, specs_sync_interval_ms=60_000)

    try:
        statsig.initialize().wait()
        for _ in range(3):
            baseline = mock_scrapi.times_called_for_endpoint(SYNC_TEST_DCS_ENDPOINT)
            process = multiprocessing.Process(target=_recover_and_sleep_task)
            process.start()
            process.join(timeout=5)
            assert process.exitcode == 0
            _wait_for_endpoint_calls(
                mock_scrapi,
                SYNC_TEST_DCS_ENDPOINT,
                baseline + EAGER_POSTFORK_PROCESS_COUNT,
            )
    finally:
        statsig.shutdown().wait()


def test_forking_inline(httpserver: HTTPServer):
    mock_scrapi = setup_server(httpserver)

    ForkableStatsigWrapper.initialize(mock_scrapi)

    timeout = 10
    proc_index = 0
    for _ in range(10):
        procs = []
        for proc_name in _proc_name_generator(start=proc_index):
            p = multiprocessing.Process(
                target=_pass_through_task,
                name=proc_name,
            )
            procs.append(p)
            p.start()

            proc_index += 1

        pid = os.getpid()

        start = time.time()
        while (time.time() - start) <= timeout:
            alive = [p.pid for p in procs if p.is_alive()]
            if not any(alive):
                break

            time.sleep(0.1)

        timed_out_procs = [p for p in procs if p.is_alive()]
        assert not any(
            timed_out_procs
        ), f"pid {pid}: timeout detected, killing all processes. Timed out procs: {timed_out_procs}"


def test_uvloop_run_forking(httpserver: HTTPServer):
    uvloop.run(_run_parent_process_after_fork(httpserver))


async def _run_parent_process_after_fork(httpserver: HTTPServer):
    mock_scrapi = setup_server(httpserver)
    statsig = _new_statsig(mock_scrapi)

    try:
        statsig.initialize().wait()
        _wait_for_next_specs_sync(mock_scrapi)

        statsig.log_event(StatsigUser("a-user"), "a_event")
        process = await asyncio.create_subprocess_exec("sleep", "0.1")
        assert await process.wait() == 0

        _assert_specs_sync_is_ticking_without_sdk_call(mock_scrapi)

        statsig.flush_events().wait()
        events = mock_scrapi.get_logged_events()
        assert events[0]["eventName"] == "a_event"
    finally:
        statsig.shutdown().wait()


def _assert_background_specs_sync_recovers_after_fork(
    mock_scrapi: MockScrapi,
) -> None:
    statsig = _new_statsig(mock_scrapi)
    process = None

    try:
        statsig.initialize().wait()
        _wait_for_next_specs_sync(mock_scrapi)

        process = multiprocessing.Process(target=_pass_through_task)
        process.start()
        process.join(timeout=5)

        assert not process.is_alive(), "forked child did not exit"
        assert process.exitcode == 0

        _assert_specs_sync_is_ticking_without_sdk_call(mock_scrapi)
    finally:
        if process is not None and process.is_alive():
            process.terminate()
            process.join(timeout=5)
        statsig.shutdown().wait()


def _new_statsig(
    mock_scrapi: MockScrapi,
    sdk_key: str = SYNC_TEST_SDK_KEY,
    specs_sync_interval_ms: int = SPECS_SYNC_INTERVAL_MS,
) -> Statsig:
    options = StatsigOptions(
        specs_url=mock_scrapi.url_for_endpoint(DCS_URL_ENDPOINT),
        log_event_url=mock_scrapi.url_for_endpoint("/v1/log_event"),
        id_lists_url=mock_scrapi.url_for_endpoint("/v1/get_id_lists"),
        specs_sync_interval_ms=specs_sync_interval_ms,
    )
    options.output_log_level = "none"
    return Statsig(sdk_key, options)


def _assert_specs_sync_is_ticking_without_sdk_call(
    mock_scrapi: MockScrapi,
) -> None:
    calls_before_recovery = mock_scrapi.times_called_for_endpoint(
        SYNC_TEST_DCS_ENDPOINT
    )

    calls_after_first_tick = _wait_for_endpoint_calls(
        mock_scrapi,
        SYNC_TEST_DCS_ENDPOINT,
        calls_before_recovery + 1,
    )

    # No SDK operation occurs after deferred recovery. Two requests prove that
    # recovery started a recurring sync loop rather than merely performing a
    # one-time fetch.
    _wait_for_endpoint_calls(
        mock_scrapi,
        SYNC_TEST_DCS_ENDPOINT,
        calls_after_first_tick + 1,
    )


def _wait_for_next_specs_sync(mock_scrapi: MockScrapi) -> int:
    current_calls = mock_scrapi.times_called_for_endpoint(SYNC_TEST_DCS_ENDPOINT)
    return _wait_for_endpoint_calls(
        mock_scrapi,
        SYNC_TEST_DCS_ENDPOINT,
        current_calls + 1,
    )


def _wait_for_gate_value(
    statsig: Statsig,
    user: StatsigUser,
    expected: bool,
    timeout_seconds: float = 3,
) -> bool:
    deadline = time.monotonic() + timeout_seconds
    while time.monotonic() < deadline:
        if statsig.check_gate(user, "test_public") is expected:
            return True
        time.sleep(0.05)
    return False


def _wait_for_endpoint_calls(
    mock_scrapi: MockScrapi,
    endpoint: str,
    minimum_calls: int,
) -> int:
    deadline = time.monotonic() + SPECS_SYNC_TIMEOUT_SECONDS
    while time.monotonic() < deadline:
        calls = mock_scrapi.times_called_for_endpoint(endpoint)
        if calls >= minimum_calls:
            return calls
        time.sleep(0.05)

    calls = mock_scrapi.times_called_for_endpoint(endpoint)
    raise AssertionError(
        f"timed out waiting for {endpoint} to reach {minimum_calls} calls; "
        f"observed {calls}"
    )


def _proc_name_generator(start: int) -> Generator:
    num_cpus = os.cpu_count() or 1
    num_concurrent_workers = max(num_cpus - 1, 1)
    process_name_pattern = "my_process_{suffix}"
    for suffix in range(start, start + num_concurrent_workers):
        yield process_name_pattern.format(suffix=suffix)


def _pass_through_task() -> None:
    """Do nothing."""
    pass


def _sleep_task() -> None:
    """Keep the child alive long enough for its async refresh to begin."""
    time.sleep(1)


def _recover_and_sleep_task() -> None:
    if sys.platform != "darwin":
        assert recover_after_fork()
    time.sleep(1)


def _evaluate_while_postfork_refresh_is_blocked(
    statsig: Statsig,
    user: StatsigUser,
    allow_evaluation,
    result_sender,
) -> None:
    if not recover_after_fork():
        result_sender.close()
        return

    if not allow_evaluation.wait(timeout=5):
        result_sender.close()
        return

    evaluation_started = time.monotonic()
    value = statsig.check_gate(user, "test_public")
    result_sender.send((value, time.monotonic() - evaluation_started))

    refreshed = _wait_for_gate_value(statsig, user, False)
    result_sender.send(False if refreshed else None)
    result_sender.close()


class ForkableStatsigWrapper:
    _at_fork_hooks_registered = False
    _log_event_url = None
    _id_lists_url = None
    _specs_url = None

    @classmethod
    def initialize(cls, mock_scrapi: MockScrapi) -> None:
        if not cls._at_fork_hooks_registered:
            os.register_at_fork(
                before=cls.maybe_shutdown_statsig,
                after_in_parent=cls._initialize_statsig,
                after_in_child=cls._initialize_statsig,
            )
            cls._at_fork_hooks_registered = True

        cls._log_event_url = mock_scrapi.url_for_endpoint("/v1/log_event")
        cls._id_lists_url = mock_scrapi.url_for_endpoint("/v1/get_id_lists")
        cls._specs_url = mock_scrapi.url_for_endpoint("/v2/download_config_specs")

        cls._initialize_statsig()

    @classmethod
    def maybe_shutdown_statsig(cls) -> None:
        if Statsig.has_shared_instance():
            pid = os.getpid()
            start = datetime.datetime.now()

            Statsig.shared().shutdown().wait()
            Statsig.remove_shared()

            elapsed = round((datetime.datetime.now() - start).total_seconds(), 4)
            assert (
                elapsed < 5
            ), f"pid {pid}: shutdown took too long, completed in {elapsed}s"

    @classmethod
    def _initialize_statsig(cls) -> None:
        cls.maybe_shutdown_statsig()

        pid = os.getpid()

        start = datetime.datetime.now()

        options = StatsigOptions()
        options.specs_url = cls._specs_url
        options.log_event_url = cls._log_event_url
        options.id_lists_url = cls._id_lists_url
        options.output_log_level = "none"

        shared_instance = Statsig.new_shared("secret-forking-test", options)
        shared_instance.initialize().wait()

        elapsed = round((datetime.datetime.now() - start).total_seconds(), 4)
        assert elapsed < 5, f"pid {pid}: Init took too long, completed in {elapsed}s"
