<?php

declare(strict_types=1);

namespace Statsig\Tests;

use PHPUnit\Framework\TestCase;

class MockServerTest extends TestCase
{
    protected MockServer $server;

    protected function setUp(): void
    {
        $this->server = new MockServer();
        $this->server->mock('/foo', 'RESULT');
    }

    protected function tearDown(): void
    {
        $this->server->stop();
    }

    public function testGetRequest()
    {
        file_get_contents($this->server->getUrl() . '/foo?buzz=1');

        $request = $this->server->getRequests()[0];
        $this->assertEquals('/foo', $request['path']);
        $this->assertEquals('GET', $request['method']);
        $this->assertEquals(['buzz' => '1'], $request['params']);
    }

    public function testPostRequest()
    {
        $context = stream_context_create([
            'http' => [
                'method' => 'POST',
                'header' => 'Content-Type: application/json',
                'content' => json_encode(['buzz' => '1'])
            ]
        ]);
        file_get_contents($this->server->getUrl() . '/foo', false, $context);

        $request = $this->server->getRequests()[0];
        $this->assertEquals('/foo', $request['path']);
        $this->assertEquals('POST', $request['method']);
        $this->assertEquals('{"buzz":"1"}', $request['body']);
        $this->assertArrayHasKey('Content-Type', $request['headers']);
        $this->assertEquals('application/json', $request['headers']['Content-Type']);
    }
}
