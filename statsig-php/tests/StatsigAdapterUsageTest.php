<?php

declare(strict_types=1);

namespace Statsig\Tests;

use PHPUnit\Framework\TestCase;
use Statsig\StatsigScheduledSpecsAdapter;
use Statsig\StatsigOptions;
use Statsig\Statsig;
use Statsig\StatsigUser;

class StatsigAdapterUsageTest extends TestCase
{
    protected MockServer $server;

    protected function setUp(): void
    {
        parent::setUp();

        if (file_exists("/tmp/specs.json")) {
            unlink("/tmp/specs.json");
        }

        $dir = dirname(__FILE__);
        $data = file_get_contents($dir . '/../../statsig-lib/tests/data/eval_proj_dcs.json');

        $this->server = new MockServer();
        $this->server->mock('/v2/download_config_specs/secret-key.json', $data);
    }

    public function testScheduledSpecsAdapterUsage()
    {
        $options = new StatsigOptions(
            $this->server->getUrl() . "/v2/download_config_specs",
            $this->server->getUrl() . "/v1/log_event"
        );

        $adapter = new StatsigScheduledSpecsAdapter("/tmp/specs.json", "secret-key", $options);
        $adapter->sync_specs_from_network();

        $options = new StatsigOptions(
            null,
            null,
            $adapter
        );


        $statsig = new Statsig("secret-key", $options);
        $statsig->initialize(function () use (&$callback_fired) {
            $callback_fired = true;
        });

        TestHelpers::waitUntilTrue($this, function () use (&$callback_fired) {
            return $callback_fired;
        });

        $user = new StatsigUser("a-user");
        $gate = $statsig->getFeatureGate("test_50_50", $user);
        $this->assertTrue($gate->value);
    }
}
