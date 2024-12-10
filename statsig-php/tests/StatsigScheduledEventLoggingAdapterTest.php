<?php

declare(strict_types=1);

namespace Statsig\Tests;

use PHPUnit\Framework\TestCase;
use Statsig\StatsigLocalFileEventLoggingAdapter;
use Statsig\StatsigOptions;


class StatsigScheduledEventLoggingAdapterTest extends TestCase
{
    const FILE_PATH = "/tmp/2454024434_events.json"; // djb2(SDK_KEY)_events.json
    const SDK_KEY = "secret-php-local-events-test";

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
        $request_json = json_encode([
            "requests" => [
                [
                    "payload" => [
                        "events" => [
                            [
                                "eventName" => "my_custom_event",
                                "metadata" => null,
                                "secondaryExposures" => null,
                                "time" => 1730831508904,
                                "user" => [
                                    "statsigEnvironment" => null,
                                    "userID" => "a-user",
                                ],
                                "value" => null,
                            ],
                        ],
                        "statsigMetadata" => [
                            "sdkType" => "statsig-server-core",
                            "sdkVersion" => "0.0.1",
                            "sessionId" => "1ff863ed-a9ab-4785-bb0e-1a7b0140c040",
                        ],
                    ],
                    "eventCount" => 1,]
            ]
        ]);

        file_put_contents(self::FILE_PATH, $request_json);

        $adapter = new StatsigLocalFileEventLoggingAdapter(
            self::SDK_KEY,
            "/tmp",
            $this->server->getUrl() . "/v1/log_event"
        );

        $adapter->send_pending_events();

        $request = $this->server->getRequests()[0];
        $this->assertEquals('/v1/log_event', $request['path']);
    }
}
