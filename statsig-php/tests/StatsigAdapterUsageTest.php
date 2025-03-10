<?php

declare(strict_types=1);

namespace Statsig\Tests;

use PHPUnit\Framework\TestCase;
use Statsig\StatsigLocalFileSpecsAdapter;
use Statsig\StatsigOptions;
use Statsig\Statsig;
use Statsig\StatsigLocalFileEventLoggingAdapter;
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
        $data = file_get_contents($dir . '/../../statsig-rust/tests/data/eval_proj_dcs.json');

        $this->server = new MockServer();
        $this->server->mock('/v2/download_config_specs/secret-key.json', $data);
        $this->server->mock('/v2/download_config_specs/server-event-logging-usage-test.json', $data);
    }

    public function testLocalFileSpecsAdapterUsage()
    {
        $adapter = new StatsigLocalFileSpecsAdapter(
            "secret-key",
            "/tmp",
            $this->server->getUrl() . "/v2/download_config_specs"
        );
        $adapter->syncSpecsFromNetwork();

        $options = new StatsigOptions(
            null,
            null,
            $adapter
        );

        $statsig = new Statsig("secret-key", $options);
        $statsig->initialize();

        $user = new StatsigUser("a-user");
        $gate = $statsig->getFeatureGate($user, "test_50_50");
        $this->assertTrue($gate->value);
    }

    public function testEventLogging()
    {
        $sdk_key = "server-event-logging-usage-test";
        $specs_adapter = new StatsigLocalFileSpecsAdapter(
            $sdk_key,
            "/tmp",
            $this->server->getUrl() . "/v2/download_config_specs"
        );

        $specs_adapter->syncSpecsFromNetwork();

        $event_adapter = new StatsigLocalFileEventLoggingAdapter($sdk_key, "/tmp");

        $statsig = new Statsig($sdk_key, new StatsigOptions(null, null, $specs_adapter, $event_adapter));

        $statsig->initialize();

        $this->assertTrue(true);
    }
}
