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

    public function logEvents(LogEventRequest $request): bool
    {
        $this->logEventsCalled = true;
        $this->lastRequest = $request;
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
