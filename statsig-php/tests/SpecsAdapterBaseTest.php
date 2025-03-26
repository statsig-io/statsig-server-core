<?php

declare(strict_types=1);

namespace Statsig\Tests;

use PHPUnit\Framework\TestCase;
use Statsig\SpecsAdapterBase;
use Statsig\SpecsUpdateListener;
use Statsig\StatsigUser;
use Statsig\StatsigOptions;
use Statsig\Statsig;

class MockSpecsAdapter extends SpecsAdapterBase
{
    public $setup_called = false;
    public $start_called = false;
    public $shutdown_called = false;
    public $schedule_background_sync_called = false;

    public ?SpecsUpdateListener $listener = null;

    public function setup(SpecsUpdateListener $listener)
    {
        $this->listener = $listener;
    }

    public function start()
    {
        $timestamp = intval(microtime(true) * 1000);
        $dir = dirname(__FILE__);
        $data = file_get_contents($dir . '/../../statsig-rust/tests/data/eval_proj_dcs.json');

        $this->listener->didReceiveSpecsUpdate($data, "Mock", $timestamp);
    }

    public function shutdown()
    {
        $this->shutdown_called = true;
    }

    public function scheduleBackgroundSync()
    {
        $this->schedule_background_sync_called = true;
    }
}

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
}
