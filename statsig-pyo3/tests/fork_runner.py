from statsig_python_core import (
    Statsig,
    StatsigOptions,
    StatsigUser,
    # pre_fork,
    # post_fork,
)
import os
import sys


def create_statsig(specs_url, log_event_url, id_lists_url):
    options = StatsigOptions()
    options.output_log_level = "debug"

    options.disable_country_lookup = True
    options.disable_user_agent_parsing = True

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


def fork_and_wait(depth):
    # pre_fork()
    pid = os.fork()
    # post_fork()

    if pid == 0:
        # Test Child
        child_statsig = create_statsig(specs_url, log_event_url, id_lists_url)
        child_statsig.initialize().wait()
        check = child_statsig.check_gate(StatsigUser("a-user"), "test_public")
        child_statsig.shutdown().wait()

        if depth >= 0:
            fork_and_wait(depth - 1)

        if check:
            sys.exit(0)

        sys.exit(1)

    pid_done, status = os.waitpid(pid, 0)
    assert pid_done == pid
    assert status == 0


for _ in range(10):
    # Test Parent
    statsig = create_statsig(specs_url, log_event_url, id_lists_url)
    statsig.initialize().wait()
    assert statsig.check_gate(StatsigUser("a-user"), "test_public")
    statsig.shutdown().wait()

    print("...Forking...")

    fork_and_wait(3)
