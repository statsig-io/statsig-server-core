<?php

namespace Statsig\Tests;

use PHPUnit\Framework\TestCase;
use Statsig\Statsig;
use Statsig\StatsigOptions;
use Statsig\StatsigUser;

/**
 * @runTestsInSeparateProcesses
 */
class ShutdownCyclingTest extends TestCase
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

    public function testShutdownCyclingFirst()
    {
        $this->runShutdownCycle(0);
    }

    public function testShutdownCyclingSecond()
    {
        $this->runShutdownCycle(1);
    }

    public function testShutdownCyclingThird()
    {
        $this->runShutdownCycle(2);
    }

    public function testShutdownCyclingFourth()
    {
        $this->runShutdownCycle(3);
    }

    public function testShutdownCyclingFifth()
    {
        $this->runShutdownCycle(4);
    }

    private function runShutdownCycle(int $iteration)
    {
        $options = new StatsigOptions(
            specs_url: $this->server->getUrl() . '/v2/download_config_specs',
            log_event_url: $this->server->getUrl() . '/v1/log_event',
            output_log_level: 'none',
        );

        $statsig = new Statsig('secret-key', $options);
        $statsig->initialize();

        $user = new StatsigUser('test_user');

        $gate = $statsig->getFeatureGate($user, 'test_public');
        $this->assertTrue($gate->value);

        if ($iteration % 2 === 0) {
            $statsig->shutdown();
        }
    }
}
