from statsig_python_core import StatsigBasePy, StatsigOptions
from requests import request
from typing import Optional, Dict, Tuple
from .error_boundary import ErrorBoundary


def network_func(method: str, url: str, headers: dict, bytes: bytes) -> Tuple[
    int, Optional[bytes], Optional[str], Optional[Dict[str, str]]]:
    try:
        response = request(method=method, url=url, headers=headers, data=bytes)
        status_code = response.status_code
        data = response.content
        headers = dict(response.headers)

        return (status_code, data, None, headers)
    except Exception as e:
        return (0, None, str(e), None)


class Statsig(StatsigBasePy):
    _statsig_shared_instance = None

    def __new__(cls, sdk_key: str, options: Optional[StatsigOptions] = None):
        instance = super().__new__(cls, network_func, sdk_key, options)
        ErrorBoundary.wrap(instance)
        return instance

    # ----------------------------
    #       Shared Instance
    # ----------------------------

    @classmethod
    def shared(cls) -> StatsigBasePy:
        if not Statsig.has_shared_instance() or cls._statsig_shared_instance is None:
            return create_statsig_error_instance(
                "Statsig.shared() called, but no instance has been set with Statsig.new_shared(...)")

        return cls._statsig_shared_instance

    @classmethod
    def new_shared(cls, sdk_key: str, options: Optional[StatsigOptions] = None) -> StatsigBasePy:
        if Statsig.has_shared_instance():
            return create_statsig_error_instance(
                "Statsig shared instance already exists. Call Statsig.remove_shared() before creating a new instance.")

        cls._statsig_shared_instance = super().__new__(cls, network_func, sdk_key, options)
        return cls._statsig_shared_instance

    @classmethod
    def remove_shared(cls) -> None:
        cls._statsig_shared_instance = None

    @classmethod
    def has_shared_instance(cls) -> bool:
        return hasattr(cls, '_statsig_shared_instance') and cls._statsig_shared_instance is not None


def create_statsig_error_instance(message: str) -> StatsigBasePy:
    print("Error: ", message)
    return StatsigBasePy.__new__(StatsigBasePy, network_func, "__STATSIG_ERROR_SDK_KEY__", None)
