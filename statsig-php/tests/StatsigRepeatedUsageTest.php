<?php

declare(strict_types=1);

namespace Statsig\Tests;

use PHPUnit\Framework\TestCase;
use Statsig\Statsig;
use Statsig\StatsigOptions;
use Statsig\StatsigUserBuilder;
use Statsig\StatsigLocalFileSpecsAdapter;
use Statsig\StatsigLocalFileEventLoggingAdapter;

const SDK_KEY = "secret-repeated-usage-test";
const TEST_DIR = "/tmp/php-repeated-usage-test";

class StatsigRepeatedUsageTest extends TestCase
{
    protected MockServer $server;
    protected Statsig $statsig;

    protected function setUp(): void
    {
        parent::setUp();

        $dir = dirname(__FILE__);
        $data = file_get_contents($dir . '/../../statsig-lib/tests/data/eval_proj_dcs.json');

        $this->server = new MockServer();
        $this->server->mock('/v2/download_config_specs/' . SDK_KEY . '.json', $data);
        $this->server->mock('/v1/log_event', '{ "success": true }', ['status' => 202]);

        TestHelpers::ensureEmptyDir(TEST_DIR);

        $adapter = new StatsigLocalFileSpecsAdapter(SDK_KEY, TEST_DIR, $this->server->getUrl() . "/v2/download_config_specs");
        $adapter->syncSpecsFromNetwork();
    }

    protected function tearDown(): void
    {
        $this->server->stop();
    }

    protected function getNewStatsigInstance(): Statsig
    {
        $options = new StatsigOptions(
            null,
            null,
            new StatsigLocalFileSpecsAdapter(SDK_KEY, TEST_DIR, $this->server->getUrl() . "/v2/download_config_specs"),
            new StatsigLocalFileEventLoggingAdapter(SDK_KEY, TEST_DIR, $this->server->getUrl() . "/v1/log_event")
        );

        $statsig = new Statsig(SDK_KEY, $options);
        $statsig->initialize();
        return $statsig;
    }

    public function testRepeatedUsage()
    {
        $iterations = 10;
        for ($i = 0; $i < $iterations; $i++) {
            $statsig = $this->getNewStatsigInstance();

            $user = StatsigUserBuilder::withUserID("user-" . $i)->build();

            $statsig->checkGate($user, "test_public");
            unset($statsig);
        }

        $logging_adapter = new StatsigLocalFileEventLoggingAdapter(SDK_KEY, TEST_DIR, $this->server->getUrl() . "/v1/log_event");
        $logging_adapter->sendPendingEvents();

        // logged each exposure
        $this->assertCount($iterations, $this->server->getLoggedEvents());

        // did not include a diagnostics event with each exposure
        $this->assertLessThan($iterations * 2, count($this->server->getLoggedEvents(true)));
    }
}
