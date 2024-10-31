<?php

declare(strict_types=1);

namespace Statsig\Tests;

use PHPUnit\Framework\TestCase;
use Statsig\Statsig;
use Statsig\StatsigOptions;
use Statsig\StatsigUser;

class StatsigTest extends TestCase
{
    protected StatsigUser $user;
    protected MockServer $server;

    protected function setUp(): void
    {
        parent::setUp();

        $this->user = new StatsigUser("a-user");

        $dir = dirname(__FILE__);
        $data = file_get_contents($dir . '/../../statsig-lib/tests/data/eval_proj_dcs.json');

        $this->server = new MockServer();
        $this->server->mock('/v2/download_config_specs/secret-key.json', $data);
        $this->server->mock('/v1/log_event', '{ "success": true }', ['status' => 202]);
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
        $options = new StatsigOptions(
            $this->server->getUrl() . "/v2/download_config_specs",
            $this->server->getUrl() . "/v1/log_event"
        );
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

    public function testCheckGate() {
        $statsig = $this->getInitializedStatsig();
        $this->assertTrue($statsig->checkGate("test_public", $this->user));
    }

    public function testGetFeatureGate() {
        $statsig = $this->getInitializedStatsig();

        $gate = $statsig->getFeatureGate("test_50_50", $this->user);
        $this->assertTrue($gate->value);
    }

    public function testGetDynamicConfig() {
        $statsig = $this->getInitializedStatsig();

        $config = $statsig->getDynamicConfig("test_email_config", $this->user);
        $this->assertEquals("everyone else", $config->get("header_text", "err"));
    }

    public function testGetExperiment() {
        $statsig = $this->getInitializedStatsig();

        $experiment = $statsig->getExperiment("exp_with_obj_and_array", $this->user);
        $this->assertEquals(["group" => "test"], $experiment->get("obj_param", ["fallback" => ""]));
    }

    public function testGetLayer() {
        $statsig = $this->getInitializedStatsig();

        $layer = $statsig->getLayer("layer_with_many_params", $this->user);
        $this->assertEquals("layer", $layer->get("a_string", "err"));
    }

    public function testExposureLogs() {
        $statsig = $this->getInitializedStatsig();

        $statsig->getFeatureGate("test_50_50", $this->user);
        $statsig->getDynamicConfig("test_email_config", $this->user);
        $statsig->getExperiment("exp_with_obj_and_array", $this->user);
        $statsig->getLayer("layer_with_many_params", $this->user)->get("a_string", "");


        $this->waitUntilEventsFlushed($statsig);

        $request = $this->server->getRequests()[1];
        $this->assertEquals('/v1/log_event', $request['path']);

        $events = json_decode($request['body'], true)['events'];
        $this->assertCount(4, $events);
    }

    public function testGcir()
    {
        $statsig = $this->getInitializedStatsig();

        $raw_result = $statsig->getClientInitializeResponse($this->user);
        $result = json_decode($raw_result, true);

        $this->assertCount(62, $result["dynamic_configs"]);
        $this->assertCount(65, $result["feature_gates"]);
        $this->assertCount(12, $result["layer_configs"]);
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

    protected function waitUntilEventsFlushed(Statsig $statsig): void
    {
        $callback_fired = false;
        $statsig->flushEvents(function () use (&$callback_fired) {
            $callback_fired = true;
        });

        $this->waitUntilTrue(function () use (&$callback_fired) {
            return $callback_fired;
        });
    }
}
