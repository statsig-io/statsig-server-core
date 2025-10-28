<?php

declare(strict_types=1);

namespace Statsig\Tests;

use PHPUnit\Framework\TestCase;
use Statsig\StatsigUser;
use Statsig\StatsigOptions;
use Statsig\Statsig;

class SpecsAdapterBaseTest extends TestCase
{
    protected StatsigUser $user;
    protected MockServer $server;
    protected MockSpecsAdapter $adapter;
    protected Statsig $statsig;

    protected function setUp(): void
    {
        parent::setUp();

        $this->user = new StatsigUser('a-user');

        $this->server = new MockServer();
        $this->server->mock('/v2/download_config_specs/secret-key.json', "{}", ['status' => 500]);
        $this->server->mock('/v1/log_event', '{ "success": true }', ['status' => 202]);

        $this->adapter = new MockSpecsAdapter();
        $options = new StatsigOptions(
            null,
            $this->server->getUrl() . '/v1/log_event',
            $this->adapter,
        );

        $this->statsig = new Statsig('secret-key', $options);
        $this->statsig->initialize();
    }


    protected function tearDown(): void
    {
        $this->statsig->flushEvents();
        $this->server->stop();
    }

    public function testListenerIsSet()
    {
        $this->assertTrue($this->adapter->listener !== null);
    }

    public function testAdapterHasCorrectName()
    {
        $gate = $this->statsig->checkGate($this->user, "test_public");
        $this->assertTrue($gate);


        $this->statsig->flushEvents();
        $events = $this->server->getLoggedEvents();

        ['metadata' => $metadata] = $events[0];

        $this->assertEquals('Adapter(Mock):Recognized', $metadata['reason']);
    }

    public function testBackgroundSyncIsScheduled()
    {
        $this->assertTrue($this->adapter->schedule_background_sync_called);
    }

    public function testShutdownIsCalled()
    {
        $this->statsig->shutdown();
        $this->assertTrue($this->adapter->shutdown_called);
    }
}
