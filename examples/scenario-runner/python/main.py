import json
import time
import requests
from typing import TypedDict, List

from statsig_wrapper import StatsigWrapper

SCRAPI_URL = "http://scrapi:8000"


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
        res = requests.get(f"{SCRAPI_URL}/v2/download_config_specs/xx")
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


def update():
    print("--------------------------------------- [ Update ]")

    state = read_sdk_state()

    print(f'Users: {len(state["users"])}')
    print(f'Gates: count({len(state["gate"]["names"])}) qps({state["gate"]["qps"]})')
    print(
        f'Events: count({len(state["logEvent"]["events"])}) qps({state["logEvent"]["qps"]})'
    )

    for user_data in state["users"]:
        StatsigWrapper.set_user(user_data)

        for gate_name in state["gate"]["names"]:
            for _ in range(state["gate"]["qps"]):
                StatsigWrapper.check_gate(gate_name)

        for event in state["logEvent"]["events"]:
            for _ in range(state["logEvent"]["qps"]):
                StatsigWrapper.log_event(event["eventName"])


# Run update every second
while True:
    update()
    time.sleep(1)
