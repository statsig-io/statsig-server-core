<?php

namespace Statsig;

class StatsigLocalFileEventLoggingAdapter
{
    public ?string $__ref = null; // phpcs:ignore

    public function __construct(
        string $sdk_key,
        string $output_directory,
        ?string $log_event_url = null,
        bool $disable_networking = false
    ) {
        $this->__ref = StatsigFFI::get()->statsig_local_file_event_logging_adapter_create(
            $sdk_key,
            $output_directory,
            $log_event_url,
            $disable_networking
        );
    }

    public function __destruct()
    {
        if (!is_null($this->__ref)) {
            StatsigFFI::get()->statsig_local_file_event_logging_adapter_release($this->__ref);
        }

        $this->__ref = null;
    }

    public function sendPendingEvents(): void
    {
        StatsigFFI::get()->statsig_local_file_event_logging_adapter_send_pending_events($this->__ref);
    }
}
