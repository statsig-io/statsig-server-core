# Celery prefork integration test

This test initializes `Statsig` once while Celery imports its application in
the master process, then uses that inherited instance from real prefork
workers. It covers both stable workers and `--max-tasks-per-child=1` recycling.

The lifecycle assertions distinguish the two post-fork phases explicitly:

- Python's raw `after_in_child` hook only marks recovery as pending. It does
  not restart background work or make requests from the unsafe fork callback.
- Celery's `worker_process_init` signal performs deferred recovery in each
  Linux worker before the first task can use the inherited SDK instance.

Each task reports its worker PID and the observed recovery calls. The test
requires exactly one successful `worker_process_init` recovery per worker,
checks that no task starts before recovery completes, and verifies exactly one
PID-tagged event is flushed for every task. The ordinary fork suite separately
covers lazy recovery when no Celery signal is available.

The test is intentionally separate from the default Python suite because it
requires Celery and Redis. Run it on Linux with the repository's locally built
Python package installed:

```bash
STATSIG_RUN_CELERY_PREFORK_TESTS=1 \
python -m pytest -v tests/integration/celery_prefork
```

If `STATSIG_CELERY_BROKER_URL` is unset, the test starts a local
`redis-server` on an ephemeral port. Otherwise it uses the configured Redis
URL, which is suitable for a CI service container.

The Python bindings job runs this integration suite explicitly on Linux after
building and installing the local wheel.
