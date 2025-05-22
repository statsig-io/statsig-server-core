from statsig_python_core import StatsigBasePy, StatsigOptions, notify_python_shutdown
from typing import Optional
from .error_boundary import ErrorBoundary
import atexit


def handle_atexit():
    notify_python_shutdown()


atexit.register(handle_atexit)


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


def create_statsig_error_instance(message: str) -> StatsigBasePy:
    print("Error: ", message)
    return StatsigBasePy.__new__(StatsigBasePy, "__STATSIG_ERROR_SDK_KEY__", None)
