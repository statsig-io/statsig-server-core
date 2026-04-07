import os
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
from typing import Optional
from .error_boundary import ErrorBoundary
from .statsig_types import DynamicConfig, FeatureGate, Experiment, Layer
import atexit


def handle_atexit():
    notify_python_shutdown()


atexit.register(handle_atexit)


def handle_fork():
    notify_python_fork()


if hasattr(os, "register_at_fork"):
    os.register_at_fork(
        before=handle_fork,
    )


class Statsig(StatsigBasePy):
    _statsig_shared_instance = None

    def __new__(cls, sdk_key: str, options: Optional[StatsigOptions] = None):
        instance = super().__new__(cls, sdk_key, options)
        ErrorBoundary.wrap(instance)
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

    def get_layer(
        self,
        user: StatsigUser,
        name: str,
        options: Optional[LayerEvaluationOptions] = None,
    ) -> Layer:
        raw = super()._INTERNAL_get_layer(user, name, options)
        return Layer(
            lambda param: self._INTERNAL_log_layer_param_exposure(raw, param), name, raw
        )


def create_statsig_error_instance(message: str) -> StatsigBasePy:
    print("Error: ", message)
    return StatsigBasePy.__new__(StatsigBasePy, "__STATSIG_ERROR_SDK_KEY__", None)
