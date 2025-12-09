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

    public function testGetLayer()
    {
        $statsig = $this->getInitializedStatsig();

        $layer = $statsig->getLayer($this->user, 'layer_with_many_params');
        $this->assertEquals('layer', $layer->get('a_string', 'err'));
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
        $this->assertMatchesRegularExpression('/\w{8}-\w{4}-\w{4}-\w{4}-\w{12}/', $session_id);
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
