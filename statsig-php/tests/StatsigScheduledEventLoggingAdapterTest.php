<?php

declare(strict_types=1);

namespace Statsig\Tests;

use PHPUnit\Framework\TestCase;
use Statsig\StatsigLocalFileEventLoggingAdapter;

class StatsigScheduledEventLoggingAdapterTest extends TestCase
{
    private const FILE_PATH = "/tmp/2454024434_events.json"; // djb2(SDK_KEY)_events.json
    private const SDK_KEY = "secret-php-local-events-test";

    protected MockServer $server;

    protected function setUp(): void
    {
        parent::setUp();

        if (file_exists(self::FILE_PATH)) {
            unlink(self::FILE_PATH);
        }

        $this->server = new MockServer();
        $this->server->mock('/v1/log_event', '{"success": true}');
    }

    public function testCreateAndRelease()
    {
        $adapter = new StatsigLocalFileEventLoggingAdapter(self::SDK_KEY, "/tmp");
        $this->assertNotNull($adapter->__ref);

        $adapter->__destruct();

        $this->assertNull($adapter->__ref);
    }

    public function testSendingEvents()
    {
        $request_json = json_encode([[
            "eventName" => "foo",
            "metadata" => ["key" => "value"],
            "secondaryExposures" => null,
            "time" => 1734476293616,
            "user" => ["statsigEnvironment" => null, "userID" => "a-user"],
            "value" => "bar"
        ]]);

        file_put_contents(self::FILE_PATH, $request_json);

        $adapter = new StatsigLocalFileEventLoggingAdapter(
            self::SDK_KEY,
            "/tmp",
            $this->server->getUrl() . "/v1/log_event"
        );

        $adapter->sendPendingEvents();

        $request = $this->server->getRequests()[0];
        $this->assertEquals('/v1/log_event', $request['path']);
    }
}
