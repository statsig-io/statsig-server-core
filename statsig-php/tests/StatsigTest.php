<?php

declare(strict_types=1);

namespace Statsig\Tests;

use PHPUnit\Framework\TestCase;
use Statsig\Statsig;
use Statsig\StatsigOptions;
use Statsig\StatsigUser;

class StatsigTest extends TestCase
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

    public function testCreateAndRelease()
    {
        $statsig = new Statsig('secret-key');
        $this->assertNotNull($statsig->__ref);

        $statsig->__destruct();

        $this->assertNull($statsig->__ref);
    }

    public function testDoubleRelease()
    {
        $statsig = new Statsig('secret-key');
        $statsig->__destruct();
        $statsig->__destruct();

        $this->assertNull($statsig->__ref);
    }

    public function testInitialization()
    {
        $statsig = $this->getInitializedStatsig();
        $this->assertEquals(Statsig::class, get_class($statsig));

        $request = $this->server->getRequests()[0];
        $this->assertEquals('/v2/download_config_specs/secret-key.json', $request['path']);
    }

    public function testCheckGate()
    {
        $statsig = $this->getInitializedStatsig();
        $this->assertTrue($statsig->checkGate($this->user, 'test_public'));
    }

    public function testGetFeatureGate()
    {
        $statsig = $this->getInitializedStatsig();

        $gate = $statsig->getFeatureGate($this->user, 'test_50_50');
        $this->assertTrue($gate->value);
    }

    public function testGetDynamicConfig()
    {
        $statsig = $this->getInitializedStatsig();

        $config = $statsig->getDynamicConfig($this->user, 'test_email_config');
        $this->assertEquals('everyone else', $config->get('header_text', 'err'));
    }

    public function testGetExperiment()
    {
        $statsig = $this->getInitializedStatsig();

        $experiment = $statsig->getExperiment($this->user, 'exp_with_obj_and_array');
        $this->assertEquals(['group' => 'test'], $experiment->get('obj_param', ['fallback' => '']));
    }

    public function testGetExperimentByGroupName()
    {
        $statsig = $this->getInitializedStatsig();

        $control = $statsig->getExperimentByGroupName('test_experiment_no_targeting', 'Control');
        $this->assertEquals('Control', $control->groupName);
        $this->assertEquals('54QJztEPRLXK7ZCvXeY9q4', $control->rule_id);
        $this->assertEquals('userID', $control->id_type);
        $this->assertEquals(['value' => 'control'], $control->value);

        $test = $statsig->getExperimentByGroupName('test_experiment_no_targeting', 'Test');
        $this->assertEquals('Test', $test->groupName);
        $this->assertEquals(['value' => 'test_1'], $test->value);
    }

    public function testGetExperimentByGroupNameUnrecognized()
    {
        $statsig = $this->getInitializedStatsig();

        $experiment = $statsig->getExperimentByGroupName('not_an_experiment', 'Control');
        $this->assertNull($experiment->groupName);
        $this->assertEquals('', $experiment->rule_id);
    }

    public function testGetExperimentByGroupIdAdvanced()
    {
        $statsig = $this->getInitializedStatsig();

        $experiment = $statsig->getExperimentByGroupIdAdvanced(
            'test_experiment_no_targeting',
            '54QJztEPRLXK7ZCvXeY9q4'
        );
        $this->assertEquals('Control', $experiment->groupName);
        $this->assertEquals('54QJztEPRLXK7ZCvXeY9q4', $experiment->rule_id);
        $this->assertEquals(['value' => 'control'], $experiment->value);
    }

    public function testGetExperimentByGroupIdAdvancedUnrecognized()
    {
        $statsig = $this->getInitializedStatsig();

        $experiment = $statsig->getExperimentByGroupIdAdvanced('test_experiment_no_targeting', 'not_a_group_id');
        $this->assertNull($experiment->groupName);
        $this->assertEquals('', $experiment->rule_id);
    }

    public function testGetLayer()
    {
        $statsig = $this->getInitializedStatsig();

        $layer = $statsig->getLayer($this->user, 'layer_with_many_params');
        $this->assertEquals('layer', $layer->get('a_string', 'err'));
    }

    public function testGetExperimentGroups()
    {
        $statsig = $this->getInitializedStatsig();

        $result = $statsig->getExperimentGroups('test_experiment_no_targeting');

        $this->assertTrue($result->isExperimentActive);

        $groups_by_name = [];
        foreach ($result->groups as $group) {
            $groups_by_name[$group->groupName] = $group;
        }

        // Only the experiment group rules are returned (the layerAssignment rule is excluded).
        $names = array_keys($groups_by_name);
        sort($names);
        $this->assertEquals(['Control', 'Test', 'Test2'], $names);
        $this->assertEquals(['value' => 'control'], $groups_by_name['Control']->returnValue);
        $this->assertEquals('54QJztEPRLXK7ZCvXeY9q4', $groups_by_name['Control']->ruleId);
        $this->assertEquals('userID', $groups_by_name['Control']->idType);
        $this->assertEquals(['value' => 'test_1'], $groups_by_name['Test']->returnValue);
        $this->assertEquals(['value' => 'test_2'], $groups_by_name['Test2']->returnValue);
    }

    public function testGetExperimentGroupsReturnsNullActiveStateForUnknownExperiment()
    {
        $statsig = $this->getInitializedStatsig();

        $result = $statsig->getExperimentGroups('nonexistent_experiment');
        $this->assertNull($result->isExperimentActive);
        $this->assertEquals([], $result->groups);
    }

    public function testGetExperimentGroupsReturnsNullActiveStateForDynamicConfig()
    {
        $statsig = $this->getInitializedStatsig();

        $result = $statsig->getExperimentGroups('test_max_dynamic_config_size_again');
        $this->assertNull($result->isExperimentActive);
        $this->assertEquals([], $result->groups);
    }

    public function testGetExperimentGroupsReturnsGroupsForInactiveExperiment()
    {
        $statsig = $this->getInitializedStatsig();

        // test_switchback has isActive: false; groups are still returned along with the flag.
        $result = $statsig->getExperimentGroups('test_switchback');
        $this->assertFalse($result->isExperimentActive);

        // Only the experiment group rules are returned (non-group rules are excluded).
        $group_names = array_map(fn ($group) => $group->groupName, $result->groups);
        sort($group_names);
        $this->assertEquals(['Control', 'Test'], $group_names);
    }

    public function testExposureLogCounts()
    {
        $statsig = $this->getInitializedStatsig();

        $statsig->getFeatureGate($this->user, 'test_50_50');
        $statsig->getDynamicConfig($this->user, 'test_email_config');
        $statsig->getExperiment($this->user, 'exp_with_obj_and_array');
        $statsig->getLayer($this->user, 'layer_with_many_params')->get('a_string', '');

        $statsig->flushEvents();

        $request = $this->server->getRequests()[1];
        $this->assertEquals('/v1/log_event', $request['path']);

        $bytes = $request['body'];
        $json = gzdecode($bytes);
        $body = json_decode($json, true);
        $events = array_filter($body['events'], function ($event) {
            return $event['eventName'] !== 'statsig::diagnostics';
        });
        $this->assertCount(4, $events);
    }

    public function testEventFormat()
    {
        $statsig = $this->getInitializedStatsig();

        $statsig->getFeatureGate($this->user, 'test_50_50');
        $statsig->flushEvents();

        $request = $this->server->getRequests()[1];

        $bytes = $request['body'];
        $json = gzdecode($bytes);
        $body = json_decode($json, true);

        $events = $this->server->getLoggedEvents();

        // Event
        [
            'eventName' => $event_name,
            'time' => $time,
            'user' => $user,
            'value' => $value,
            'metadata' => $metadata
        ] = $events[0];
        $this->assertEquals('statsig::gate_exposure', $event_name);
        $this->assertIsNumeric($time);
        $this->assertEquals('a-user', $user['userID']);
        $this->assertEquals('true', $metadata['gateValue']);
        $this->assertEquals('test_50_50', $metadata['gate']);
        $this->assertEquals('Network:Recognized', $metadata['reason']);

        // Statsig Metadata
        [
            'sdkType' => $sdk_type,
            'sdkVersion' => $sdk_version,
            'sessionID' => $session_id,
            'os' => $os,
            'arch' => $arch,
            'languageVersion' => $lang_version
        ] = $body['statsigMetadata'];
        $this->assertEquals('statsig-server-core-php', $sdk_type);
        $this->assertIsString($sdk_version);
        $this->assertIsString($session_id);
        $this->assertIsString($os);
        $this->assertIsString($arch);
        $this->assertIsString($lang_version);
    }

    public function testGcir()
    {
        $statsig = $this->getInitializedStatsig();

        $raw_result = $statsig->getClientInitializeResponse($this->user);
        $result = json_decode($raw_result, true);

        $this->assertCount(65, $result['dynamic_configs']);
        $this->assertCount(69, $result['feature_gates']);
        $this->assertCount(12, $result['layer_configs']);
    }
}
