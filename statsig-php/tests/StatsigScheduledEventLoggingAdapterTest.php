<?php

declare(strict_types=1);

namespace Statsig\Tests;

use PHPUnit\Framework\TestCase;
use Statsig\StatsigScheduledEventLoggingAdapter;
use Statsig\StatsigOptions;

class StatsigScheduledEventLoggingAdapterTest extends TestCase
{
    protected MockServer $server;

    protected function setUp(): void
    {
        parent::setUp();

        if (file_exists("/tmp/events.json")) {
            unlink("/tmp/events.json");
        }

        $this->server = new MockServer();
        $this->server->mock('/v1/log_event', '{"success": true}');
    }

    public function testCreateAndRelease()
    {
        $adapter = new StatsigScheduledEventLoggingAdapter("/tmp/events.json", "secret-key");
        $this->assertNotNull($adapter->__http_ref);

        $adapter->__destruct();

        $this->assertNull($adapter->__http_ref);
    }

    public function testSendingEvents()
    {
        $options = new StatsigOptions(
            $this->server->getUrl() . "__unused__",
            $this->server->getUrl() . "/v1/log_event"
        );

        $file_path = "/tmp/events.json";
        $request_json = json_encode([
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
            "eventCount" => 1,
        ]);

        file_put_contents($file_path, $request_json);

        $adapter = new StatsigScheduledEventLoggingAdapter($file_path, "secret-key", $options);

        $result = null;
        $adapter->send_pending_events(function ($success, $err_msg) use (&$result) {
            $result = [
                'success' => $success,
                'err_msg' => $err_msg,
            ];
        });

        TestHelpers::waitUntilTrue($this, function () use (&$result) {
            return !is_null($result);
        });

        $this->assertNull($result['err_msg']);

        $request = $this->server->getRequests()[0];
        $this->assertEquals('/v1/log_event', $request['path']);
    }
}
