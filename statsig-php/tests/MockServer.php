<?php

namespace Statsig\Tests;

use donatj\MockWebServer\MockWebServer;
use donatj\MockWebServer\Response;
use donatj\MockWebServer\Responses\NotFoundResponse;

// donatj\MockWebServer docs: https://github.com/donatj/mock-webserver/blob/master/docs/docs.md

class MockServer
{
    private MockWebServer $server;

    public function __construct()
    {
        $this->server = new MockWebServer;
        $this->server->setDefaultResponse(new NotFoundResponse);

        $this->server->start();
    }

    public function stop(): void
    {
        $this->server->stop();
    }

    public function getUrl(): string
    {
        return $this->server->getServerRoot();
    }

    public function mock($path, $response, $options = []): void
    {
        $status = $options['status'] ?? 200;

        $this->server->setResponseOfPath(
            $path,
            new Response(
                $response,
                ['Cache-Control' => 'no-cache'],
                $status
            )
        );
    }

    public function getRequests(): array
    {
        $requests = [];

        for ($i = 0; $i < 999; $i++) {
            $request = $this->server->getRequestByOffset($i);
            if ($request) {
                $requests[] = [
                    'uri' =>  $request->getRequestUri(),
                    'method' => $request->getRequestMethod(),
                    'params' => $request->getGet(),
                    'body' => $request->getInput(),
                    'headers' => $request->getHeaders(),
                    'path' => $request->getParsedUri()['path']
                ];
            } else {
                break;
            }
        }

        return $requests;
    }
}
