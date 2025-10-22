<?php

namespace Statsig;

abstract class EventLoggingAdapterBase
{
    public $__ref = null; // phpcs:ignore

    public function __construct()
    {
        $ffi = StatsigFFI::get();
        $this->__ref = $ffi->function_based_event_logging_adapter_create(
            "php",
            [$this, 'start'],
            fn($request) => $this->logEvents(new LogEventRequest($request)),
            [$this, 'shutdown'],
        );
    }

    public function __destruct()
    {
        if (is_null($this->__ref)) {
            return;
        }

        $ffi = StatsigFFI::get();
        $ffi->function_based_event_logging_adapter_release($this->__ref);
        $this->__ref = null;
    }

    abstract public function start();
    abstract public function logEvents(LogEventRequest $request): bool;
    abstract public function shutdown();
}
