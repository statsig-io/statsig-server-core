import os
import time
from typing import Any

from celery import Celery
from celery.signals import worker_process_init


_worker_init_started = False
_worker_init_completed = False
_lifecycle_phase = "master"
_recovery_calls: list[dict[str, Any]] = []


@worker_process_init.connect(weak=False)
def _mark_worker_init_started(**_kwargs: Any) -> None:
    global _worker_init_started, _lifecycle_phase
    _worker_init_started = True
    _lifecycle_phase = "worker_process_init"


# Importing the SDK after the marker connects its recovery receiver after the
# start marker. A completion marker is connected below after SDK setup.
from statsig_python_core import Statsig, StatsigOptions, StatsigUser


BROKER_URL = os.environ["CELERY_BROKER_URL"]
SDK_KEY = "secret-celery-prefork"

app = Celery("statsig-celery-prefork", broker=BROKER_URL, backend=BROKER_URL)
app.conf.update(
    result_expires=60,
    task_serializer="json",
    result_serializer="json",
    accept_content=["json"],
)

options = StatsigOptions(
    specs_url=os.environ["STATSIG_TEST_SPECS_URL"],
    log_event_url=os.environ["STATSIG_TEST_LOG_EVENT_URL"],
    id_lists_url=os.environ["STATSIG_TEST_ID_LISTS_URL"],
    specs_sync_interval_ms=1000,
)
options.output_log_level = "none"


class InstrumentedStatsig(Statsig):
    def _INTERNAL_restart_background_tasks_after_fork(
        self, timeout_ms: int = 3000
    ) -> bool:
        call = {
            "pid": os.getpid(),
            "phase": _lifecycle_phase,
            "success": False,
            "timeout_ms": timeout_ms,
        }
        _recovery_calls.append(call)
        success = super()._INTERNAL_restart_background_tasks_after_fork(timeout_ms)
        call["success"] = success
        return success


# This is deliberately constructed and initialized while Celery is importing
# the application in its master process, before the prefork pool is created.
statsig = InstrumentedStatsig(SDK_KEY, options)
statsig.initialize().wait()


@worker_process_init.connect(weak=False)
def _mark_worker_init_completed(**_kwargs: Any) -> None:
    global _worker_init_completed, _lifecycle_phase
    _worker_init_completed = True
    _lifecycle_phase = "ready"


@app.task(name="statsig_tests.evaluate_and_log")
def evaluate_and_log(task_index: int, expected_gate_value: bool) -> dict[str, object]:
    worker_pid = os.getpid()
    task_started_before_recovery = not _worker_init_completed
    user = StatsigUser(f"celery-user-{task_index}")
    deadline = time.monotonic() + 10
    gate_value = statsig.check_gate(user, "test_public")
    while gate_value is not expected_gate_value and time.monotonic() < deadline:
        time.sleep(0.05)
        gate_value = statsig.check_gate(user, "test_public")
    statsig.log_event(
        user,
        "celery-prefork-event",
        value=f"{task_index}:{worker_pid}",
    )
    # A recycled worker exits as soon as this task returns. Keep it alive for
    # several configured 100 ms logger ticks so this scenario verifies the
    # recovered automatic flush loop rather than racing process teardown.
    time.sleep(1)
    return {
        "task_index": task_index,
        "gate_value": gate_value,
        "expected_gate_value": expected_gate_value,
        "worker_pid": worker_pid,
        "worker_init_started": _worker_init_started,
        "worker_init_completed": _worker_init_completed,
        "task_started_before_recovery": task_started_before_recovery,
        "recovery_calls": [
            call for call in _recovery_calls if call["pid"] == worker_pid
        ],
    }
