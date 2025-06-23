<?php

declare(strict_types=1);

namespace Statsig\Tests;

use PHPUnit\Framework\TestCase;
use Statsig\Statsig;
use Statsig\StatsigOptions;
use Statsig\StatsigUser;

class StatsigLocalOverridesTest extends TestCase
{
    protected StatsigUser $user;
    protected MockServer $server;

    protected function setUp(): void
    {
        parent::setUp();

        $this->user = new StatsigUser('a-user');

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

    protected function getInitializedStatsig(): Statsig
    {
        $options = new StatsigOptions(
            $this->server->getUrl() . '/v2/download_config_specs',
            $this->server->getUrl() . '/v1/log_event'
        );
        $statsig = new Statsig('secret-key', $options);

        $statsig->initialize();

        return $statsig;
    }

    public function testOverrideGate()
    {
        $statsig = $this->getInitializedStatsig();

        $statsig->overrideGate('test_public', false);

        $gate = $statsig->getFeatureGate($this->user, 'test_public');
        $this->assertFalse($gate->value);
        $this->assertEquals('override', $gate->rule_id);
    }

    public function testOverrideDynamicConfig()
    {
        $statsig = $this->getInitializedStatsig();

        $statsig->overrideDynamicConfig('operating_system_config', ['foo' => 'bar']);

        $config = $statsig->getDynamicConfig($this->user, 'operating_system_config');
        $this->assertEquals(['foo' => 'bar'], $config->value);
        $this->assertEquals('override', $config->rule_id);
    }

    public function testOverrideExperiment()
    {
        $statsig = $this->getInitializedStatsig();

        $statsig->overrideExperiment('test_exp_random_id', ['foo' => 'bar']);

        $experiment = $statsig->getExperiment($this->user, 'test_exp_random_id');
        $this->assertEquals(['foo' => 'bar'], $experiment->value);
        $this->assertEquals('override', $experiment->rule_id);
    }

    public function testOverrideExperimentByGroupName()
    {
        $statsig = $this->getInitializedStatsig();

        $statsig->overrideExperimentByGroupName('test_exp_random_id', 'Control');

        $experiment = $statsig->getExperiment($this->user, 'test_exp_random_id');
        $this->assertEquals(['bool' => false, 'string' => 'control', 'num' => 999], $experiment->value);
        $this->assertEquals('override', $experiment->rule_id);
    }

    public function testOverrideLayer()
    {
        $statsig = $this->getInitializedStatsig();

        $statsig->overrideLayer('test_layer', ['foo' => 'bar']);

        $layer = $statsig->getLayer($this->user, 'test_layer');
        $this->assertEquals('bar', $layer->get('foo', 'err'));
        $this->assertEquals('override', $layer->rule_id);
    }

    public function testOverrideGateForUser()
    {
        $statsig = $this->getInitializedStatsig();

        $statsig->overrideGate('test_public', false, 'a-user');

        $gate = $statsig->getFeatureGate($this->user, 'test_public');
        $this->assertFalse($gate->value);
        $this->assertEquals('override', $gate->rule_id);
    }

    public function testOverrideDynamicConfigForUser()
    {
        $statsig = $this->getInitializedStatsig();

        $statsig->overrideDynamicConfig('operating_system_config', ['foo' => 'bar'], 'a-user');

        $config = $statsig->getDynamicConfig($this->user, 'operating_system_config');
        $this->assertEquals(['foo' => 'bar'], $config->value);
        $this->assertEquals('override', $config->rule_id);
    }

    public function testOverrideExperimentForUser()
    {
        $statsig = $this->getInitializedStatsig();

        $statsig->overrideExperiment('test_exp_random_id', ['foo' => 'bar'], 'a-user');

        $experiment = $statsig->getExperiment($this->user, 'test_exp_random_id');
        $this->assertEquals(['foo' => 'bar'], $experiment->value);
        $this->assertEquals('override', $experiment->rule_id);
    }

    public function testOverrideExperimentByGroupNameForUser()
    {
        $statsig = $this->getInitializedStatsig();

        $statsig->overrideExperimentByGroupName('test_exp_random_id', 'Control', 'a-user');

        $experiment = $statsig->getExperiment($this->user, 'test_exp_random_id');
        $this->assertEquals(['bool' => false, 'string' => 'control', 'num' => 999], $experiment->value);
        $this->assertEquals('override', $experiment->rule_id);
    }

    public function testOverrideLayerForUser()
    {
        $statsig = $this->getInitializedStatsig();

        $statsig->overrideLayer('test_layer', ['foo' => 'bar'], 'a-user');

        $layer = $statsig->getLayer($this->user, 'test_layer');
        $this->assertEquals('bar', $layer->get('foo', 'err'));
        $this->assertEquals('override', $layer->rule_id);
    }

    public function testRemoveGateOverride()
    {
        $statsig = $this->getInitializedStatsig();

        $statsig->overrideGate('test_public', false);
        $statsig->removeGateOverride('test_public');

        $gate = $statsig->getFeatureGate($this->user, 'test_public');
        $this->assertTrue($gate->value);
        $this->assertNotEquals('override', $gate->rule_id);
    }

    public function testRemoveDynamicConfigOverride()
    {
        $statsig = $this->getInitializedStatsig();

        $statsig->overrideDynamicConfig('test_email_config', ['foo' => 'bar']);
        $statsig->removeDynamicConfigOverride('test_email_config');

        $config = $statsig->getDynamicConfig($this->user, 'test_email_config');
        $this->assertEquals('everyone else', $config->get('header_text', 'err'));
        $this->assertNotEquals('override', $config->rule_id);
    }

    public function testRemoveExperimentOverride()
    {
        $statsig = $this->getInitializedStatsig();

        $statsig->overrideExperiment('exp_with_obj_and_array', ['foo' => 'bar']);
        $statsig->removeExperimentOverride('exp_with_obj_and_array');

        $experiment = $statsig->getExperiment($this->user, 'exp_with_obj_and_array');
        $this->assertEquals(['group' => 'test'], $experiment->get('obj_param', ['fallback' => '']));
        $this->assertNotEquals('override', $experiment->rule_id);
    }

    public function testRemoveLayerOverride()
    {
        $statsig = $this->getInitializedStatsig();

        $statsig->overrideLayer('layer_with_many_params', ['foo' => 'bar']);
        $statsig->removeLayerOverride('layer_with_many_params');

        $layer = $statsig->getLayer($this->user, 'layer_with_many_params');
        $this->assertEquals('layer', $layer->get('a_string', 'err'));
        $this->assertNotEquals('override', $layer->rule_id);
    }

    public function testRemoveAllOverrides()
    {
        $statsig = $this->getInitializedStatsig();

        $statsig->overrideGate('test_public', false);
        $statsig->overrideDynamicConfig('test_email_config', ['foo' => 'bar']);
        $statsig->overrideExperiment('exp_with_obj_and_array', ['foo' => 'bar']);

        $statsig->removeAllOverrides();

        $gate = $statsig->getFeatureGate($this->user, 'test_public');
        $this->assertTrue($gate->value);
        $this->assertNotEquals('override', $gate->rule_id);

        $config = $statsig->getDynamicConfig($this->user, 'test_email_config');
        $this->assertEquals('everyone else', $config->get('header_text', 'err'));
        $this->assertNotEquals('override', $config->rule_id);

        $experiment = $statsig->getExperiment($this->user, 'exp_with_obj_and_array');
        $this->assertEquals(['group' => 'test'], $experiment->get('obj_param', ['fallback' => '']));
        $this->assertNotEquals('override', $experiment->rule_id);
    }

    public function testOverridePriority()
    {
        $statsig = $this->getInitializedStatsig();

        $statsig->overrideGate('test_public', false);

        $statsig->overrideGate('test_public', true, 'a-user');

        $gate = $statsig->getFeatureGate($this->user, 'test_public');
        $this->assertTrue($gate->value);
        $this->assertEquals('override', $gate->rule_id);
    }
}
