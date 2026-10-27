import atexit
import json
import os
import time
import weakref
from statsig_python_core import (
    DynamicConfigEvaluationOptions,
    ExperimentEvaluationOptions,
    FeatureGateEvaluationOptions,
    LayerEvaluationOptions,
    StatsigBasePy,
    StatsigOptions,
    StatsigUser,
    notify_python_fork,
    notify_python_shutdown,
)
from typing import Any, Callable, Literal, Optional
from .error_boundary import ErrorBoundary
from .statsig_types import (
    DynamicConfig,
    Experiment,
    ExperimentGroupsResult,
    FeatureGate,
    Layer,
)

_live_statsig_instances: weakref.WeakSet[Any] = weakref.WeakSet()
_child_recovery_pending = False
# Celery allows only a short worker-process initialization window. Apply one
# budget to the whole callback rather than spending a full Rust-side timeout
# on every live instance. Instances not reached remain recoverable lazily.
POST_FORK_RECOVERY_BUDGET_SECONDS = 3.0

SdkEventName = Literal[
    "*",
    "gate_evaluated",
    "dynamic_config_evaluated",
    "experiment_evaluated",
    "layer_evaluated",
    "specs_updated",
]


def handle_atexit():
    notify_python_shutdown()


atexit.register(handle_atexit)


def handle_fork():
    notify_python_fork()


def _recover_live_instances_after_fork() -> bool:
    deadline = time.monotonic() + POST_FORK_RECOVERY_BUDGET_SECONDS
    recovered = True
    for instance in list(_live_statsig_instances):
        remaining_seconds = deadline - time.monotonic()
        if remaining_seconds <= 0:
            return False
        try:
            remaining_ms = max(1, int(remaining_seconds * 1000))
            recovered = (
                instance._INTERNAL_restart_background_tasks_after_fork(remaining_ms)
                and recovered
            )
        except Exception:
            # Fork recovery must never prevent the parent or child from
            # continuing. Every SDK operation retains a lazy retry path.
            recovered = False
    return recovered


def handle_after_fork_in_parent():
    recover_after_fork()


def handle_after_fork_in_child():
    # The process has only the thread that called fork(). Do not touch SDK
    # locks, create a runtime, start threads, or perform network work here.
    global _child_recovery_pending
    _child_recovery_pending = True


def recover_after_fork() -> bool:
    """Recover live SDK instances from a safe post-fork lifecycle hook.

    Prefork frameworks without built-in integration should call this after the
    child process is fully created. Recovery is PID-aware and idempotent, and
    immediate DCS refresh is scheduled without waiting for network I/O.
    """
    global _child_recovery_pending
    recovered = _recover_live_instances_after_fork()
    if recovered:
        _child_recovery_pending = False
    return recovered


# Compatibility for users of the draft-only helper name. Both names invoke
# the same platform-neutral recovery routine.
recover_after_fork_in_child = recover_after_fork


def handle_worker_process_init(**_kwargs: Any) -> None:
    # Celery emits this in the fully-created worker child before it accepts
    # tasks, where starting threads and scheduling async work is safe.
    recover_after_fork()


def _register_celery_worker_process_init() -> None:
    try:
        from celery.signals import worker_process_init  # type: ignore[import-not-found]
    except ImportError:
        return

    worker_process_init.connect(handle_worker_process_init, weak=False)


if hasattr(os, "register_at_fork"):
    # Quiesce Tokio before forking so the child cannot inherit locks held by
    # vanished worker threads. The raw child callback only marks recovery as
    # pending; Celery worker initialization or an SDK call performs recovery.
    os.register_at_fork(
        before=handle_fork,
        after_in_parent=handle_after_fork_in_parent,
        after_in_child=handle_after_fork_in_child,
    )


_register_celery_worker_process_init()


class Statsig(StatsigBasePy):
    _statsig_shared_instance = None

    def __new__(cls, sdk_key: str, options: Optional[StatsigOptions] = None):
        instance = super().__new__(cls, sdk_key, options)
        ErrorBoundary.wrap(instance)
        _live_statsig_instances.add(instance)
        return instance

    # ----------------------------
    #       Shared Instance
    # ----------------------------

    @classmethod
    def shared(cls) -> StatsigBasePy:
        if not Statsig.has_shared_instance() or cls._statsig_shared_instance is None:
            return create_statsig_error_instance(
                "Statsig.shared() called, but no instance has been set with Statsig.new_shared(...)"
            )

        return cls._statsig_shared_instance

    @classmethod
    def new_shared(
        cls, sdk_key: str, options: Optional[StatsigOptions] = None
    ) -> StatsigBasePy:
        if Statsig.has_shared_instance():
            return create_statsig_error_instance(
                "Statsig shared instance already exists. Call Statsig.remove_shared() before creating a new instance."
            )

        cls._statsig_shared_instance = super().__new__(cls, sdk_key, options)
        _live_statsig_instances.add(cls._statsig_shared_instance)
        return cls._statsig_shared_instance

    @classmethod
    def remove_shared(cls) -> None:
        cls._statsig_shared_instance = None

    @classmethod
    def has_shared_instance(cls) -> bool:
        return (
            hasattr(cls, "_statsig_shared_instance")
            and cls._statsig_shared_instance is not None
        )

    # ------------------------------------------------------------ [ SDK Events ]

    def subscribe(
        self, event_name: SdkEventName, callback: Callable[[Any], None]
    ) -> str:
        """Subscribes to an SDK event, returning a subscription id.

        The callback is invoked on an SDK background thread with the event as a
        dict: `{"event_name": ..., "data": {...}}`. Pass `"*"` to receive every
        event. Remove the subscription with `unsubscribe_by_id`, `unsubscribe`
        or `unsubscribe_all`.
        """

        def on_event(raw: str) -> None:
            ErrorBoundary._capture("subscribe", lambda: callback(json.loads(raw)))

        return super()._INTERNAL_subscribe(event_name, on_event)

    # ------------------------------------------------------------ [ Core APIs ]

    def get_feature_gate(
        self,
        user: StatsigUser,
        name: str,
        options: Optional[FeatureGateEvaluationOptions] = None,
    ) -> FeatureGate:
        raw = super()._INTERNAL_get_feature_gate(user, name, options)
        return FeatureGate(name, raw)

    def get_dynamic_config(
        self,
        user: StatsigUser,
        name: str,
        options: Optional[DynamicConfigEvaluationOptions] = None,
    ) -> DynamicConfig:
        raw = super()._INTERNAL_get_dynamic_config(user, name, options)
        return DynamicConfig(name, raw)

    def get_experiment(
        self,
        user: StatsigUser,
        name: str,
        options: Optional[ExperimentEvaluationOptions] = None,
    ) -> Experiment:
        raw = super()._INTERNAL_get_experiment(user, name, options)
        return Experiment(name, raw)

    def get_experiment_by_group_name(
        self,
        experiment_name: str,
        group_name: str,
    ) -> Experiment:
        raw = super()._INTERNAL_get_experiment_by_group_name(
            experiment_name, group_name
        )
        return Experiment(experiment_name, raw)

    def get_experiment_by_group_id_advanced(
        self,
        experiment_name: str,
        group_id: str,
    ) -> Experiment:
        raw = super()._INTERNAL_get_experiment_by_group_id_advanced(
            experiment_name, group_id
        )
        return Experiment(experiment_name, raw)

    def get_experiment_groups(self, experiment_name: str) -> ExperimentGroupsResult:
        raw = super()._INTERNAL_get_experiment_groups(experiment_name)
        return ExperimentGroupsResult(raw)

    def get_layer(
        self,
        user: StatsigUser,
        name: str,
        options: Optional[LayerEvaluationOptions] = None,
    ) -> Layer:
        raw = super()._INTERNAL_get_layer(user, name, options)
        exposure = raw.get("__exposure") if isinstance(raw, dict) else None

        def exposure_func(param: str):
            if exposure is None:
                return
            return self._INTERNAL_log_layer_param_exposure(exposure, param)

        return Layer(
            exposure_func,
            name,
            raw,
        )


def create_statsig_error_instance(message: str) -> StatsigBasePy:
    print("Error: ", message)
    return StatsigBasePy.__new__(StatsigBasePy, "__STATSIG_ERROR_SDK_KEY__", None)
