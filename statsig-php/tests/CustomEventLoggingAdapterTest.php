<?php

namespace Statsig\Tests;

use PHPUnit\Framework\TestCase;
use Statsig\Statsig;
use Statsig\StatsigOptions;
use Statsig\StatsigUser;
use Statsig\StatsigEventData;

class CustomEventLoggingAdapterTest extends TestCase
{
    protected MockServer $server;
    protected StatsigUser $user;

    protected function setUp(): void
    {
        parent::setUp();

        $dir = dirname(__FILE__);
        $data = file_get_contents($dir . '/../../statsig-rust/tests/data/eval_proj_dcs.json');

        $this->user = new StatsigUser('a-user');
        $this->server = new MockServer();
        $this->server->mock('/v2/download_config_specs/secret-key.json', $data);
    }


    protected function tearDown(): void
    {
        $this->server->stop();
    }

    public function testCustomLoggerIsCalled()
    {
        $adapter = new MockEventLoggingAdapter();
        $options = new StatsigOptions(
            specs_url: $this->server->getUrl() . '/v2/download_config_specs',
            event_logging_adapter: $adapter,
        );

        $statsig = new Statsig('secret-key', $options);
        $statsig->initialize();

        $statsig->logEvent(new StatsigEventData("test_event", 1, ['test_key' => 'test_value']), $this->user);

        $statsig->flushEvents();

        $this->assertTrue($adapter->logEventsCalled);

        $events = $adapter->lastRequest->payload->events;
        $event = array_filter($events, fn($event) => $event['eventName'] === 'test_event');
        $event = array_values($event)[0];
        $this->assertNotEmpty($event);
        $this->assertEquals($event['value'], 1);
        $this->assertEquals($event['metadata']['test_key'], 'test_value');
    }

    public function testLoggingManyEvents()
    {
        $adapter = new MockEventLoggingAdapter();
        $adapter->excludeDiagnostics = true;
        $options = new StatsigOptions(
            specs_url: $this->server->getUrl() . '/v2/download_config_specs',
            event_logging_adapter: $adapter,
            event_logging_max_queue_size: 10
        );

        $statsig = new Statsig('secret-key', $options);
        $statsig->initialize();

        for ($i = 0; $i < 100; $i++) {
            $statsig->logEvent(new StatsigEventData("test_event", $i, ['test_key' => 'test_value']), $this->user);
        }

        $statsig->shutdown();

        $this->assertTrue($adapter->logEventsCalled);

        $events = $adapter->loggedEvents;

        $this->assertEquals(100, count($events));
    }
}
