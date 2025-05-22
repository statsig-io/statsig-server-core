from statsig_python_core import (
    Statsig,
    StatsigOptions,
    StatsigUser,
    ObservabilityClient,
)
import sys
import time


class MockObservabilityClient(ObservabilityClient):
    def init(self) -> None:
        pass

    def increment(self, metric_name, value, tags) -> None:
        pass

    def gauge(self, metric_name, value, tags) -> None:
        pass

    def dist(self, metric_name, value, tags) -> None:
        pass

    def error(self, tag, error) -> None:
        print(f"Error callback for {tag}: {error}")

    def should_enable_high_cardinality_for_this_tag(self, tag):
        return True


options = StatsigOptions()
options.enable_id_lists = True
# options.event_logging_flush_interval_ms = 1
# options.id_lists_sync_interval_ms = 1
# options.specs_sync_interval_ms = 1
options.observability_client = MockObservabilityClient()

options.disable_network = True

statsig = Statsig("secret-key", options)
statsig.initialize().wait()

user = StatsigUser("user-1")

print("!!!!!!!!!!!!!!!!!!!!!!!!!!!!Logging events")

for i in range(99999):
    statsig.log_event(user, "test_event", {"test": "test"})


print("!!!!!!!!!!!!!!!!!!!!!!!!!!!!Shutting down")
# waiter = statsig.flush_events()
waiter2 = statsig.shutdown()

time.sleep(0.1)
# print("Killing process")
# sys.exit(0)

# waiter2.wait()
