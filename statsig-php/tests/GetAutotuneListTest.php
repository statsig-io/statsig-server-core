<?php

declare(strict_types=1);

namespace Statsig\Tests;

use PHPUnit\Framework\TestCase;
use Statsig\Statsig;
use Statsig\StatsigOptions;

/**
 * @runTestsInSeparateProcesses
 */
class GetAutotuneListTest extends TestCase
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

    public function testGetAutotuneList()
    {
        $options = new StatsigOptions(
            specs_url: $this->server->getUrl() . '/v2/download_config_specs',
            log_event_url: $this->server->getUrl() . '/v1/log_event'
        );

        $statsig = new Statsig('secret-key', $options);
        $statsig->initialize();

        $autotune_list = $statsig->getAutotuneList();

        $this->assertIsArray($autotune_list);
        $this->assertContains('test_autotune', $autotune_list);
        $this->assertContains('test_dub_autotune', $autotune_list);
    }
}
