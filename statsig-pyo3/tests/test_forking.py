from mock_scrapi import MockScrapi
from pytest_httpserver import HTTPServer
from utils import get_test_data_resource
import os
from typing import Generator
import multiprocessing
import time
import datetime
from statsig_python_core import Statsig, StatsigOptions
import subprocess

multiprocessing.set_start_method("fork", force=True)


def setup_server(httpserver: HTTPServer):
    mock_scrapi = MockScrapi(httpserver)
    mock_scrapi.stub("/v1/log_event", response='{"success": true}', method="POST")
    dcs_content = get_test_data_resource("eval_proj_dcs.json")
    mock_scrapi.stub(
        "/v2/download_config_specs/secret-forking-test.json",
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

    command = f"python tests/fork_runner.py {specs_url} {log_event_url} {id_lists_url}"
    print("Running command: ", command)
    proc = subprocess.Popen(
        command,
        shell=True,
        universal_newlines=True,
        env={**os.environ, "RUST_BACKTRACE": "full"},
    )
    try:
        proc.communicate(timeout=10)
    except subprocess.TimeoutExpired:
        proc.terminate()
        proc.wait()

    assert proc.returncode == 0


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


def _proc_name_generator(start: int) -> Generator:
    num_cpus = os.cpu_count() or 1
    num_concurrent_workers = max(num_cpus - 1, 1)
    process_name_pattern = "my_process_{suffix}"
    for suffix in range(start, start + num_concurrent_workers):
        yield process_name_pattern.format(suffix=suffix)


def _pass_through_task() -> None:
    """Do nothing."""
    pass


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
