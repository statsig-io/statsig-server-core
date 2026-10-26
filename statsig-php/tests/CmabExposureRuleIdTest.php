<?php

namespace Statsig\Tests;

use PHPUnit\Framework\TestCase;
use Statsig\Statsig;
use Statsig\StatsigOptions;
use Statsig\StatsigUser;

// PHP uses the typed experiment path (statsig_get_experiment -> get_experiment)
class CmabExposureRuleIdTest extends TestCase
{
    protected MockServer $server;
    protected MockEventLoggingAdapter $adapter;
    protected Statsig $statsig;

    protected function setUp(): void
    {
        parent::setUp();

        $dir = dirname(__FILE__);
        $data = file_get_contents($dir . '/../../statsig-rust/tests/data/dcs_cmab_exposure_rule_id.json');

        $this->server = new MockServer();
        $this->server->mock('/v2/download_config_specs/secret-key.json', $data);

        $this->adapter = new MockEventLoggingAdapter();
        $this->adapter->excludeDiagnostics = true;
        $options = new StatsigOptions(
            specs_url: $this->server->getUrl() . '/v2/download_config_specs',
            event_logging_adapter: $this->adapter,
        );

        $this->statsig = new Statsig('secret-key', $options);
        $this->statsig->initialize();
    }

    protected function tearDown(): void
    {
        $this->server->stop();
    }

    public function testExploreSuffixPreservedInAutomaticExposure()
    {
        $experiment = $this->statsig->getExperiment(new StatsigUser('typed-explore-user'), 'cmab_always_explore');
        $this->statsig->flushEvents();

        $this->assertStringEndsWith(':explore', $experiment->rule_id);
        $this->assertEquals($experiment->rule_id, $this->getLoggedRuleId());
    }

    public function testExploitRuleIdRemainsBareInAutomaticExposure()
    {
        $experiment = $this->statsig->getExperiment(new StatsigUser('typed-exploit-user'), 'cmab_never_explore');
        $this->statsig->flushEvents();

        $this->assertEquals('treatment-group-id', $experiment->rule_id);
        $this->assertEquals($experiment->rule_id, $this->getLoggedRuleId());
    }

    private function getLoggedRuleId(): string
    {
        $exposures = array_values(array_filter(
            $this->adapter->loggedEvents,
            fn($event) => $event['eventName'] === 'statsig::config_exposure'
        ));

        $this->assertCount(1, $exposures);
        return $exposures[0]['metadata']['ruleID'];
    }
}
