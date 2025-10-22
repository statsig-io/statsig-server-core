<?php

namespace Statsig\Tests;

use Statsig\EventLoggingAdapterBase;
use Statsig\LogEventRequest;

class MockEventLoggingAdapter extends EventLoggingAdapterBase
{
    public $logEventsCalled = false;
    public $startCalled = false;
    public $shutdownCalled = false;
    public ?LogEventRequest $lastRequest = null;

    public $loggedEvents = [];

    public $excludeDiagnostics = false;

    public function logEvents(LogEventRequest $request): bool
    {
        $this->logEventsCalled = true;
        $this->lastRequest = $request;

        foreach ($request->payload->events as $event) {
            if ($this->excludeDiagnostics && $event['eventName'] === 'statsig::diagnostics') {
                continue;
            }

            $this->loggedEvents[] = $event;
        }

        return true;
    }

    public function start(): void
    {
        $this->startCalled = true;
    }

    public function shutdown(): void
    {
        $this->shutdownCalled = true;
    }
}
