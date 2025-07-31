import json
import time
import requests
from typing import TypedDict, List, Callable
import os

from statsig_wrapper import StatsigWrapper

SCRAPI_URL = "http://scrapi:8000"
PROFILE_ARR: List[dict] = []


class GateConfig(TypedDict):
    names: List[str]
    qps: int


class LogEventConfig(TypedDict):
    events: List[dict]
    qps: int


class SdkState(TypedDict):
    users: List[dict]
    gate: GateConfig
    logEvent: LogEventConfig


# Wait for scrapi to be ready
for i in range(10):
    try:
        res = requests.get(f"{SCRAPI_URL}/ready")
        if res.status_code == 200:
            break
    except:
        pass

    print("Waiting for scrapi to be ready")
    time.sleep(1)

# Initialize Statsig
StatsigWrapper.initialize().wait()


def read_sdk_state() -> SdkState:
    with open("/shared-volume/state.json", "r") as f:
        data = json.load(f)
        return data["sdk"]


def profile(name: str, user_id: str, extra: str, qps: int, fn: Callable):
    durations: List[float] = []
    for _ in range(qps):
        start = time.perf_counter()
        fn()
        end = time.perf_counter()
        durations.append((end - start) * 1000)

    results = {
        "name": name,
        "userID": user_id,
        "extra": extra,
        "qps": qps,
    }

    if qps > 0:
        sorted_durations = sorted(durations)
        results["median"] = sorted_durations[len(sorted_durations) // 2]
        p99 = sorted_durations[int(len(sorted_durations) * 0.99)]
        results["p99"] = p99
        results["min"] = sorted_durations[0]
        max = sorted_durations[-1]
        results["max"] = max
        print(f"{name} took {p99}ms (p99), {max}ms (max)")

    PROFILE_ARR.append(results)


def update():
    print("--------------------------------------- [ Update ]")

    state = read_sdk_state()

    PROFILE_ARR.clear()

    print(f'Users: {len(state["users"])}')
    print(f'Gates: count({len(state["gate"]["names"])}) qps({state["gate"]["qps"]})')
    print(
        f'Events: count({len(state["logEvent"]["events"])}) qps({state["logEvent"]["qps"]})'
    )

    for user_data in state["users"]:
        StatsigWrapper.set_user(user_data)

        for gate_name in state["gate"]["names"]:
            profile(
                "check_gate",
                user_data["userID"],
                gate_name,
                state["gate"]["qps"],
                lambda: StatsigWrapper.check_gate(gate_name),
            )

        for event in state["logEvent"]["events"]:
            profile(
                "log_event",
                user_data["userID"],
                event["eventName"],
                state["logEvent"]["qps"],
                lambda: StatsigWrapper.log_event(event["eventName"]),
            )

        profile(
            "gcir",
            user_data["userID"],
            "",
            state["gcir"]["qps"],
            lambda: StatsigWrapper.get_client_initialize_response(),
        )

    write_profile_data()


def write_profile_data():
    pretty_json = json.dumps(PROFILE_ARR, indent=2)
    slug = f"profile-python-{'core' if StatsigWrapper.is_core else 'legacy'}"

    with open(f"/shared-volume/{slug}-temp.json", "w") as f:
        f.write(pretty_json)

    os.system(f"mv /shared-volume/{slug}-temp.json /shared-volume/{slug}.json")


# Run update every second
while True:
    update()
    time.sleep(1)
