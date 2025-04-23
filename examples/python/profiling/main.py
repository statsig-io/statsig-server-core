import time
from statsig_python_core import (
    Statsig,
    StatsigUser,
    StatsigOptions,
)

t_file = open("timeline.csv", "w")


def mark_event(event: str):
    ts = int(time.time() * 1000)
    t_file.write(f"{ts},{event}\n")


t_file.write(f"timestamp,event\n")

time.sleep(1)
mark_event("begin")


for i in range(10):
    user = StatsigUser(user_id="user_1")
    print(f"Hello, world! {user.user_id}")
    time.sleep(0.5)

mark_event("boot_statsig_start")

statsig = Statsig("secret-9IWfdzNwExEYHEW4YfOQcFZ4xreZyFkbOXHaNbPsMwW")
statsig.initialize().wait()

mark_event("boot_statsig_end")

for i in range(10):
    user = StatsigUser(user_id=f"user_{i}")
    statsig.check_gate(user, "test_public")
    time.sleep(0.5)

mark_event("flush_events_start")
statsig.flush_events().wait()
mark_event("flush_events_end")

for i in range(10):
    user = StatsigUser(user_id=f"user_{i}")
    statsig.check_gate(user, "test_public")
    time.sleep(0.5)

mark_event("get_client_initialize_response_start")
for i in range(10):
    user = StatsigUser(user_id=f"user_{i}")
    statsig.get_client_initialize_response(user)
    time.sleep(0.5)
mark_event("get_client_initialize_response_end")
