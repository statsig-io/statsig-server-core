from statsig_python_core import Statsig, StatsigOptions, StatsigUser
import os
import sys


def create_statsig(specs_url, log_event_url, id_lists_url):
    options = StatsigOptions()
    options.output_log_level = "debug"

    options.specs_url = specs_url
    options.specs_sync_interval_ms = 1

    options.log_event_url = log_event_url

    options.id_lists_url = id_lists_url
    options.id_lists_sync_interval_ms = 1
    options.enable_id_lists = True

    return Statsig("secret-forking-test", options)


print("Fork runner PID: ", os.getpid())
specs_url = sys.argv[1]
log_event_url = sys.argv[2]
id_lists_url = sys.argv[3]

for _ in range(10):
    # Test Parent
    statsig = create_statsig(specs_url, log_event_url, id_lists_url)
    statsig.initialize().wait()
    assert statsig.check_gate(StatsigUser("a-user"), "test_public")
    statsig.shutdown().wait()

    pid = os.fork()
    if pid == 0:
        # Test Child
        child_statsig = create_statsig(specs_url, log_event_url, id_lists_url)
        child_statsig.initialize().wait()
        assert child_statsig.check_gate(StatsigUser("a-user"), "test_public")
        child_statsig.shutdown().wait()
        sys.exit(0)

    pid_done, status = os.waitpid(pid, 0)

    assert pid_done == pid
    assert status == 0
