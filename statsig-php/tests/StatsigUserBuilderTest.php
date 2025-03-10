<?php

declare(strict_types=1);

namespace Statsig\Tests;

use PHPUnit\Framework\TestCase;
use Statsig\Statsig;
use Statsig\StatsigOptions;
use Statsig\StatsigUserBuilder;

class StatsigUserBuilderTest extends TestCase
{
    protected MockServer $server;
    protected Statsig $statsig;

    protected function setUp(): void
    {
        parent::setUp();

        $dir = dirname(__FILE__);
        $data = file_get_contents($dir . '/../../statsig-rust/tests/data/eval_proj_dcs.json');

        $this->server = new MockServer();
        $this->server->mock('/v2/download_config_specs/secret-key.json', $data);
        $this->server->mock('/v1/log_event', '{ "success": true }', ['status' => 202]);

        $options = new StatsigOptions(
            $this->server->getUrl() . "/v2/download_config_specs",
            $this->server->getUrl() . "/v1/log_event"
        );

        $this->statsig = new Statsig("secret-key", $options);
        $this->statsig->initialize();
    }

    protected function tearDown(): void
    {
        $this->server->stop();
    }


    public function testBuildingFullUser()
    {
        $user = StatsigUserBuilder::withUserID("a-user")
            ->withEmail("a-user@example.com")
            ->withIP("127.0.0.1")
            ->withUserAgent("Mozilla/5.0")
            ->withCountry("US")
            ->withLocale("en_US")
            ->withAppVersion("1.0.0")
            ->withCustom(["custom" => "value"])
            ->withPrivateAttributes(["private" => "value"])
            ->build();

        $this->assertTrue($this->statsig->checkGate($user, "test_public"));
        $this->statsig->flushEvents();

        $logged_user = $this->server->getLoggedEvents()[0]['user'];

        $this->assertEquals("a-user", $logged_user['userID']);
        $this->assertEquals("a-user@example.com", $logged_user['email']);
        $this->assertEquals("127.0.0.1", $logged_user['ip']);
        $this->assertEquals("Mozilla/5.0", $logged_user['userAgent']);
        $this->assertEquals("US", $logged_user['country']);
        $this->assertEquals("en_US", $logged_user['locale']);
        $this->assertEquals("1.0.0", $logged_user['appVersion']);
        $this->assertEquals(["custom" => "value"], $logged_user['custom']);
        $this->assertArrayNotHasKey('privateAttributes', $logged_user);
    }

    public function testBuildingPartialUser()
    {
        $user = StatsigUserBuilder::withCustomIDs(["employeeID" => "an_employee"])
            ->build();

        $this->assertTrue($this->statsig->checkGate($user, "test_public"));
        $this->statsig->flushEvents();

        $logged_user = $this->server->getLoggedEvents()[0]['user'];

        $this->assertEquals(["employeeID" => "an_employee"], $logged_user['customIDs']);

        $this->assertArrayNotHasKey('email', $logged_user);
        $this->assertArrayNotHasKey('ip', $logged_user);
        $this->assertArrayNotHasKey('userAgent', $logged_user);
        $this->assertArrayNotHasKey('country', $logged_user);
        $this->assertArrayNotHasKey('locale', $logged_user);
        $this->assertArrayNotHasKey('appVersion', $logged_user);
        $this->assertArrayNotHasKey('custom', $logged_user);
        $this->assertArrayNotHasKey('privateAttributes', $logged_user);
    }
}
