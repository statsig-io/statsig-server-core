<?php

declare(strict_types=1);

namespace Statsig\Tests;

use PHPUnit\Framework\TestCase;
use Statsig\Statsig;
use Statsig\StatsigOptions;
use Statsig\StatsigUser;

function get_real_memory_usage()
{
    $pid = getmypid();
    $rss_in_kb = shell_exec("ps -o rss= -p " . $pid);
    return intval(trim($rss_in_kb)) / 1024.0;
}

class MemoryLeakTest extends TestCase
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

    // Run getClientInitializeResponse + every evaluation type once per user.
    // Mirrors the workload we want to prove is leak-free.
    private function runWorkload(Statsig $statsig, int $count): void
    {
        for ($i = 0; $i < $count; $i++) {
            $user = new StatsigUser('user_' . $i);
            $res = $statsig->getClientInitializeResponse($user);
            $this->assertNotNull($res);
        }

        for ($i = 0; $i < $count; $i++) {
            $user = new StatsigUser('user_' . $i);
            $res = $statsig->getFeatureGate($user, 'test_public');
            $this->assertNotNull($res);

            $res = $statsig->getDynamicConfig($user, 'test_email_config');
            $this->assertNotNull($res);

            $res = $statsig->getExperiment($user, 'exp_with_obj_and_array');
            $this->assertNotNull($res);

            $res = $statsig->getLayer($user, 'layer_with_many_params');
            $this->assertNotNull($res);
            gc_collect_cycles();
        }
    }

    public function testMemoryLeak()
    {
        $options = new StatsigOptions(
            specs_url: $this->server->getUrl() . '/v2/download_config_specs',
            log_event_url: $this->server->getUrl() . '/v1/log_event',
            disable_all_logging: true,
        );
        $statsig = new Statsig('secret-key', $options);

        $statsig->initialize();

        // Warm-up pass.
        //
        // We measure process RSS (via `ps`) as a proxy for leaks. RSS is a
        // high-water mark: the first time we evaluate gates / build large
        // getClientInitializeResponse payloads, the allocator grows its heap
        // (e.g. glibc per-thread malloc arenas, spec-store scratch buffers) and
        // does NOT return that memory to the OS afterwards, even once the
        // objects are freed. That one-time, bounded growth is not a leak, but
        // if we took the baseline *before* it happened we would misattribute
        // it as one (this is exactly what made the test flaky in CI: the
        // baseline was captured right after initialize(), before the heavy
        // GCIR loop, so the GCIR peak counted as "growth").
        //
        // Running the full workload once first lets the allocator reach its
        // steady-state high-water mark before we record the baseline.
        $this->runWorkload($statsig, 10000);

        gc_collect_cycles();
        $initial_memory = get_real_memory_usage();
        echo "Initial memory (post warm-up): " . $initial_memory . " MB \n";

        // Measured pass: an identical workload. A genuine per-call leak grows
        // RSS in proportion to the number of calls, so a second 10k-user pass
        // would push it well past the buffer. Bounded allocator retention, on
        // the other hand, adds ~nothing here because the heap is already sized
        // from the warm-up pass.
        $this->runWorkload($statsig, 10000);

        $statsig->shutdown();
        $statsig = null;

        $final_memory = get_real_memory_usage();

        // wait for a maximum of 5 seconds for the memory to be released
        for ($i = 0; $i < 50; $i++) {
            gc_collect_cycles();

            $final_memory = get_real_memory_usage();
            if ($final_memory - $initial_memory <= 10) {
                break;
            }

            usleep(100000); // Sleep for 100ms
        }

        echo "Final memory: " . $final_memory . " MB \n";

        // Allow a 10mb buffer for the test. With the warm-up pass this measures
        // steady-state growth, not first-touch allocator expansion, so the
        // delta reflects an actual leak rather than RSS retention noise.
        $delta = $final_memory - $initial_memory;
        $this->assertTrue($delta <= 10, "Memory was not released: " . $delta . " MB");
    }
}
