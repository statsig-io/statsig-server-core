<?php

declare(strict_types=1);

namespace Statsig\Tests;

use PHPUnit\Framework\TestCase;
use Statsig\Statsig;
use Statsig\StatsigOptions;

class StatsigTest extends TestCase
{
    protected MockServer $server;

    protected function setUp(): void {
        parent::setUp();

        $this->server = new MockServer();
        $this->server->mock('/v2/download_config_specs/secret-key.json', '{}', ['status' => 566]);
    }

    protected function tearDown(): void {
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

    public function testInitialization() {
        $options = new StatsigOptions($this->server->getUrl() . "/v2/download_config_specs");
        $statsig = new Statsig("secret-key", $options);

        $callback_fired = false;

        $statsig->initialize(function() use(&$callback_fired) {
            $callback_fired = true;
        });

        $this->assertEventuallyTrue(function() use (&$callback_fired) {
            return $callback_fired;
        });

        $request = $this->server->getRequests()[0];
        $this->assertEquals('/v2/download_config_specs/secret-key.json', $request['path']);
    }

    protected function assertEventuallyTrue($callback, $timeout_secs = 1.0): void
    {
        $start = microtime(true);
        while (!$callback() && microtime(true) - $start < $timeout_secs) {
            usleep(10000); // Sleep for 10ms
        }

        $this->assertTrue($callback());
    }
}
