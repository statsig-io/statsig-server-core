<?php

declare(strict_types=1);

namespace Statsig\Tests;

use PHPUnit\Framework\TestCase;
use Statsig\StatsigLocalFileSpecsAdapter;

class StatsigLocalFileSpecsAdapterTest extends TestCase
{
    private const SDK_KEY = "secret-php-specs-adapter";
    private const FILE_NAME = "2169430312_specs.json"; // djb2(SDK_KEY)_specs.json

    protected MockServer $server;

    protected function setUp(): void
    {
        parent::setUp();

        if (file_exists("/tmp/" . self::FILE_NAME)) {
            unlink("/tmp/" . self::FILE_NAME);
        }

        $dir = dirname(__FILE__);
        $data = file_get_contents($dir . '/../../statsig-rust/tests/data/eval_proj_dcs.json');

        $this->server = new MockServer();
        $this->server->mock('/v2/download_config_specs/' . self::SDK_KEY . '.json', $data);
    }

    public function testCreateAndRelease()
    {
        $adapter = new StatsigLocalFileSpecsAdapter(self::SDK_KEY, "/tmp");
        $this->assertNotNull($adapter->__ref);

        $adapter->__destruct();

        $this->assertNull($adapter->__ref);
    }

    public function testFetchingFromNetwork()
    {
        $adapter = new StatsigLocalFileSpecsAdapter(
            self::SDK_KEY,
            "/tmp",
            $this->server->getUrl() . "/v2/download_config_specs"
        );

        $adapter->syncSpecsFromNetwork();

        $json = json_decode(file_get_contents("/tmp/" . self::FILE_NAME), true);
        $this->assertArrayHasKey("dynamic_configs", $json);
        $this->assertArrayHasKey("layer_configs", $json);
        $this->assertArrayHasKey("feature_gates", $json);
    }
}
