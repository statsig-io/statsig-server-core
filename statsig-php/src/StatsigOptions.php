<?php

namespace Statsig;

class StatsigOptions
{
    public $__ref = null; // phpcs:ignore

    public function __construct(
        ?string $specs_url = null,
        ?string $log_event_url = null,
        ?object $specs_adapter = null,
        ?object $event_logging_adapter = null,
        ?string $environment = null,
        ?int $event_logging_flush_interval_ms = null,
        ?int $event_logging_max_queue_size = null,
        ?int $specs_sync_interval_ms = null,
        ?int $output_log_level = null
    ) {
        $ffi = StatsigFFI::get();
        $this->__ref = $ffi->statsig_options_create(
            $specs_url,
            $log_event_url,
            is_null($specs_adapter) ? null : $specs_adapter->__ref,
            is_null($event_logging_adapter) ? null : $event_logging_adapter->__ref,
            $environment,
            $event_logging_flush_interval_ms ?? -1,
            $event_logging_max_queue_size ?? -1,
            $specs_sync_interval_ms ?? -1,
            $output_log_level ?? null
        );
    }

    public function __destruct()
    {
        if (is_null($this->__ref)) {
            return;
        }

        StatsigFFI::get()->statsig_options_release($this->__ref);
        $this->__ref = null;
    }
}
