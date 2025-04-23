import time
from statsig_python_core import Statsig, StatsigUser, StatsigOptions, StatsigBasePy


def mark_event(event: str):
    with open("timeline.csv", "a") as t_file:
        ts = int(time.time() * 1000)
        t_file.write(f"{ts},{event}\n")


def network_func(method: str, url: str, headers: dict, bytes: bytes):
    if "v2/download_config_specs" in url:
        mark_event("dcs_sync")

        with open("dcs_data.json", "rb") as f:
            response_bytes = f.read()

        return (200, response_bytes, None, None)

    return (500, None, None, None)


with open("timeline.csv", "w") as t_file:
    t_file.write(f"timestamp,event\n")


time.sleep(1)
mark_event("begin")


options = StatsigOptions(
    specs_sync_interval_ms=1000,
)

statsig = StatsigBasePy(network_func, sdk_key="secret-key", options=options)
statsig.initialize().wait()
mark_event("statsig_initialized")

for i in range(5):
    time.sleep(1)
    mark_event(f"sleep_{i}_end")
