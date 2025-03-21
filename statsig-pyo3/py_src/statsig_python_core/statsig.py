from statsig_python_core import StatsigBasePy, StatsigOptions
from requests import request
from typing import Optional, Dict, Tuple

def network_func(method: str, url: str, headers: dict, bytes: bytes) -> Tuple[int, Optional[bytes], Optional[str], Optional[Dict[str, str]]]:
    try:
        response = request(method=method, url=url, headers=headers, data=bytes)
        status_code = response.status_code
        data = response.content
        headers = dict(response.headers)

        return (status_code, data, None, headers)
    except Exception as e:
        return (0, None, str(e), None)


class Statsig(StatsigBasePy):
    def __new__(cls, sdk_key: str, options: StatsigOptions = None):
        return super().__new__(cls, network_func, sdk_key, options)
