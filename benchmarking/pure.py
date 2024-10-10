import os

from time import perf_counter
from statsig import statsig, StatsigUser

statsig_secret = os.environ.get('test_api_key')
statsig.initialize(statsig_secret)

user = StatsigUser("Dan")

start = perf_counter()
for lp in range(1000):
    result = statsig.get_client_initialize_response(user)
end = perf_counter()

print(result)
print(f"Duration: {(end - start) * 1000}")

