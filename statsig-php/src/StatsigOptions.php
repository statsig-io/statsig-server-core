<?php

namespace Statsig;

class StatsigOptions
{
    public $__ref = null;

    public function __construct(
        $specs_url = null,
        $log_event_url = null,
        $specs_adapter = null,
        $event_logging_adapter = null,
    ) {
        $ffi = StatsigFFI::get();
        $this->__ref = $ffi->statsig_options_create(
            $specs_url,
            $log_event_url,
            is_null($specs_adapter) ? null : $specs_adapter->__ref,
            is_null($event_logging_adapter) ? null : $event_logging_adapter->__ref
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
