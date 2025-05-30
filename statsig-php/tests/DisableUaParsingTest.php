<?php

namespace Statsig\Tests;

use PHPUnit\Framework\TestCase;
use Statsig\Statsig;
use Statsig\StatsigOptions;
use Statsig\StatsigUser;

const ANDROID_USER_AGENT = 'Mozilla/5.0 (Android 15; Mobile; SM-G556B/DS; rv:130.0) Gecko/130.0 Firefox/130.0';

/**
 * @runTestsInSeparateProcesses
 */
class DisableUaParsingTest extends TestCase
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

    public function testWhenUaParsingIsEnabled()
    {
        $options = new StatsigOptions(
            specs_url: $this->server->getUrl() . '/v2/download_config_specs',
            log_event_url: $this->server->getUrl() . '/v1/log_event',
            disable_user_agent_parsing: false,
            wait_for_user_agent_init: true,
        );

        $statsig = new Statsig('secret-key', $options);
        $statsig->initialize();

        $user = new StatsigUser(
            'test_user',
            user_agent: ANDROID_USER_AGENT
        );

        $gate = $statsig->getFeatureGate($user, 'test_many_rules');
        $this->assertEquals($gate->rule_id, '6p3sV7B05P62iIXvQnFIN9');
    }

    public function testWhenUaParsingIsDisabled()
    {
        $options = new StatsigOptions(
            specs_url: $this->server->getUrl() . '/v2/download_config_specs',
            log_event_url: $this->server->getUrl() . '/v1/log_event',
            disable_user_agent_parsing: true,
            output_log_level: 'none',
        );

        $statsig = new Statsig('secret-key', $options);
        $statsig->initialize();

        $user = new StatsigUser(
            'test_user',
            user_agent: ANDROID_USER_AGENT,
        );

        $gate = @$statsig->getFeatureGate($user, 'test_many_rules');
        $this->assertNotEquals($gate->rule_id, '6p3sV7B05P62iIXvQnFIN9');
    }
}
