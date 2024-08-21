
from time import perf_counter
from statsig import statsig, StatsigUser


statsig.initialize("secret-9IWfdzNwExEYHEW4YfOQcFZ4xreZyFkbOXHaNbPsMwW")

user = StatsigUser("Dan")

start = perf_counter()
for lp in range(1000):
    result = statsig.get_client_initialize_response(user)
end = perf_counter()

print(result)
print(f"Duration: {(end - start) * 1000}")

