<?php

declare(strict_types=1);

namespace Statsig\Tests;

use PHPUnit\Framework\TestCase;
use Statsig\Statsig;
use Statsig\StatsigOptions;
use Statsig\StatsigUser;

class StatsigTest extends TestCase
{
    protected MockServer $server;

    protected function setUp(): void
    {
        parent::setUp();

        $dir = dirname(__FILE__);
        $data = file_get_contents($dir . '/../../statsig-lib/tests/data/eval_proj_dcs.json');

        $this->server = new MockServer();
        $this->server->mock('/v2/download_config_specs/secret-key.json', $data);
    }

    protected function tearDown(): void
    {
        $this->server->stop();
    }

    public function testCreateAndRelease()
    {
        $statsig = new Statsig("secret-key");
        $this->assertNotNull($statsig->__ref);

        $statsig->__destruct();

        $this->assertNull($statsig->__ref);
    }

    public function testDoubleRelease()
    {
        $statsig = new Statsig("secret-key");
        $statsig->__destruct();
        $statsig->__destruct();

        $this->assertNull($statsig->__ref);
    }

    function getInitializedStatsig(): Statsig
    {
        $options = new StatsigOptions($this->server->getUrl() . "/v2/download_config_specs");
        $statsig = new Statsig("secret-key", $options);

        $callback_fired = false;

        $statsig->initialize(function () use (&$callback_fired) {
            $callback_fired = true;
        });

        $this->waitUntilTrue(function () use (&$callback_fired) {
            return $callback_fired;
        });
        return $statsig;
    }

    public function testInitialization()
    {
        $statsig = $this->getInitializedStatsig();
        $this->assertEquals(Statsig::class, get_class($statsig));

        $request = $this->server->getRequests()[0];
        $this->assertEquals('/v2/download_config_specs/secret-key.json', $request['path']);
    }

    public function testGcir()
    {
        $statsig = $this->getInitializedStatsig();

        $user = new StatsigUser("a-user");
        $raw_result = $statsig->getClientInitializeResponse($user);
        $result = json_decode($raw_result, true);

        $this->assertCount(61, $result["dynamic_configs"]);
        $this->assertCount(64, $result["feature_gates"]);
        $this->assertCount(11, $result["layer_configs"]);
    }

    protected function waitUntilTrue($callback, $timeout_secs = 1.0): void
    {
        $start = microtime(true);
        while (!$callback() && microtime(true) - $start < $timeout_secs) {
            usleep(10000); // Sleep for 10ms
        }

        if (!$callback()) {
            $this->fail("Timed out waiting for callback");
        }
    }
}
