<?php

namespace Statsig;

class StatsigLocalFileEventLoggingAdapter
{
    public ?string $__ref = null;

    public function __construct(
        string $sdk_key,
        string $output_directory,
        string $log_event_url = null
    ) {
        $this->__ref = StatsigFFI::get()->statsig_local_file_event_logging_adapter_create(
            $sdk_key,
            $output_directory,
            $log_event_url
        );
    }

    public function __destruct()
    {
        if (!is_null($this->__ref)) {
            StatsigFFI::get()->statsig_local_file_event_logging_adapter_release($this->__ref);
        }

        $this->__ref = null;
    }

    public function send_pending_events(): void
    {
        StatsigFFI::get()->statsig_local_file_event_logging_adapter_send_pending_events($this->__ref);
    }
}
