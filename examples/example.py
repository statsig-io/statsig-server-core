import sys
import os
from time import perf_counter

build_path = os.path.abspath(os.path.join(os.path.dirname(__file__), '../build/python/sigstat'))
sys.path.insert(0, build_path)

from statsig import Statsig, User

user_id = "Dan"
email = "daniel@statsig.com"

user = User(user_id, email)
statsig = Statsig("secret-9IWfdzNwExEYHEW4YfOQcFZ4xreZyFkbOXHaNbPsMwW")

gate_name = "test_public"

result = statsig.check_gate(user, gate_name)
print(f"Gate check {'passed' if result else 'failed'}!")

# start = perf_counter()
# for lp in range(1000):
#     user = User("user_" + str(lp), email)
#     result = statsig.get_client_init_response(user)
# end = perf_counter()
#
# print(result)
# print(f"Duration: {(end - start) * 1000}")

