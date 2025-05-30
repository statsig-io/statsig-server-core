<?php

namespace Statsig\Tests;

use PHPUnit\Framework\TestCase;
use Statsig\Statsig;
use Statsig\StatsigOptions;
use Statsig\StatsigUser;

const US_IP_ADDRESS = '4.38.114.186';

/**
 * @runTestsInSeparateProcesses
 */
class DisableCountryLookupTest extends TestCase
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

    public function testWhenCountryLookupIsEnabled()
    {
        $options = new StatsigOptions(
            specs_url: $this->server->getUrl() . '/v2/download_config_specs',
            log_event_url: $this->server->getUrl() . '/v1/log_event',
            disable_country_lookup: false,
            wait_for_country_lookup_init: true,
        );

        $statsig = new Statsig('secret-key', $options);
        $statsig->initialize();

        $user = new StatsigUser(
            'test_user',
            ip: US_IP_ADDRESS
        );

        $gate = $statsig->getFeatureGate($user, 'test_country');
        $this->assertEquals($gate->rule_id, '1yhP7ww1Ot82rjqi1kh4eR');
    }

    public function testWhenCountryLookupIsDisabled()
    {
        $options = new StatsigOptions(
            specs_url: $this->server->getUrl() . '/v2/download_config_specs',
            log_event_url: $this->server->getUrl() . '/v1/log_event',
            disable_country_lookup: true,
            output_log_level: 'none',
        );

        $statsig = new Statsig('secret-key', $options);
        $statsig->initialize();

        $user = new StatsigUser(
            'test_user',
            ip: US_IP_ADDRESS
        );

        $gate = @$statsig->getFeatureGate($user, 'test_country');
        $this->assertNotEquals($gate->rule_id, '1yhP7ww1Ot82rjqi1kh4eR');
    }
}
