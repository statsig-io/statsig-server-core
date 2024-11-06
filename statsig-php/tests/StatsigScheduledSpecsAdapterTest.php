<?php

declare(strict_types=1);

namespace Statsig\Tests;

use PHPUnit\Framework\TestCase;
use Statsig\StatsigScheduledSpecsAdapter;
use Statsig\StatsigOptions;

class StatsigScheduledSpecsAdapterTest extends TestCase
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

    public function testCreateAndRelease()
    {
        $adapter = new StatsigScheduledSpecsAdapter("/tmp/specs.json", "secret-key");
        $this->assertNotNull($adapter->__http_ref);
        $this->assertNotNull($adapter->__ref);

        $adapter->__destruct();

        $this->assertNull($adapter->__http_ref);
        $this->assertNull($adapter->__ref);
    }

    public function testFetchingFromNetwork()
    {
        $options = new StatsigOptions(
            $this->server->getUrl() . "/v2/download_config_specs",
            $this->server->getUrl() . "/v1/log_event"
        );
        $adapter = new StatsigScheduledSpecsAdapter("/tmp/specs.json", "secret-key", $options);

        $adapter->sync_specs_from_network();

        $json = json_decode(file_get_contents("/tmp/specs.json"), true);

        $this->assertArrayHasKey("dynamic_configs", $json);
        $this->assertArrayHasKey("layer_configs", $json);
        $this->assertArrayHasKey("feature_gates", $json);
    }
}
