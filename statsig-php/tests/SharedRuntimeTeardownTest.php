<?php

namespace Statsig\Tests;

use PHPUnit\Framework\TestCase;

/**
 * The shared tokio runtime outlives every Statsig instance, so releasing an
 * instance does not stop its worker threads. PHP unloads the FFI library at the
 * end of each request, and on Windows that unmaps the module while those
 * threads are still running inside it, faulting the worker.
 *
 * Statsig registers a teardown hook that stops the shared runtime. Because PHP
 * runs shutdown callbacks BEFORE object destructors, that hook has to flush
 * live instances itself -- otherwise it would stop the runtime that still owes
 * an event flush.
 *
 * These run in a child process because the behavior only happens at process
 * shutdown, after a test method has returned.
 */
class SharedRuntimeTeardownTest extends TestCase
{
    protected MockServer $server;

    protected function setUp(): void
    {
        parent::setUp();

        $dir = dirname(__FILE__);
        $data = file_get_contents($dir . '/../../statsig-rust/tests/data/eval_proj_dcs.json');

        $this->server = new MockServer();
        $this->server->mock('/v2/download_config_specs/secret-key.json', $data);
        $this->server->mock('/v1/log_event', '{ "success": true }', ['status' => 202]);
    }

    protected function tearDown(): void
    {
        $this->server->stop();
    }

    public function testFlushesEventsWhenTheUserNeverCallsShutdown(): void
    {
        $this->runChildRequest('never_shutdown');

        $this->assertContains(
            'teardown_probe',
            $this->loggedEventNames(),
            'Teardown stopped the shared runtime without flushing a live instance first'
        );
    }

    public function testFlushesEventsWhenTheUserShutsDownExplicitly(): void
    {
        $this->runChildRequest('explicit_shutdown');

        $this->assertContains('teardown_probe', $this->loggedEventNames());
    }

    public function testExitsCleanlyWithNoStatsigInstance(): void
    {
        // A request that never constructs a Statsig registers no hook at all.
        $this->runChildRequest('no_instance');

        $this->assertSame([], $this->loggedEventNames());
    }

    /**
     * @return array<int, string>
     */
    private function loggedEventNames(): array
    {
        return array_column($this->server->getLoggedEvents(), 'eventName');
    }

    private function runChildRequest(string $mode): void
    {
        $command = sprintf(
            '%s %s %s %s 2>&1',
            escapeshellarg(PHP_BINARY),
            escapeshellarg(dirname(__FILE__) . '/scripts/shared_runtime_teardown_child.php'),
            escapeshellarg($mode),
            escapeshellarg($this->server->getUrl())
        );

        $output = [];
        $exit_code = 0;
        exec($command, $output, $exit_code);

        // A crash during teardown surfaces here as a signal exit, which is the
        // failure this whole hook exists to prevent.
        $this->assertSame(0, $exit_code, "Child request did not exit cleanly:\n" . implode("\n", $output));
    }
}
