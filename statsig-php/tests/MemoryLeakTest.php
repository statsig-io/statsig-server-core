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

    public function testMemoryLeak()
    {
        $options = new StatsigOptions(
            specs_url: $this->server->getUrl() . '/v2/download_config_specs',
            log_event_url: $this->server->getUrl() . '/v1/log_event',
            disable_all_logging: true,
        );
        $statsig = new Statsig('secret-key', $options);

        $statsig->initialize();

        $initial_memory = get_real_memory_usage();
        echo "Initial memory: " . $initial_memory . " MB \n";

        for ($i = 0; $i < 10000; $i++) {
            $user = new StatsigUser('user_' . $i);
            $res = $statsig->getClientInitializeResponse($user);
            $this->assertNotNull($res);
        }

        for ($i = 0; $i < 10000; $i++) {
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

        // Allow a 10mb buffer for the test
        $delta = $final_memory - $initial_memory;
        $this->assertTrue($delta <= 10, "Memory was not released: " . $delta . " MB");
    }
}
