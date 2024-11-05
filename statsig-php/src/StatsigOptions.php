<?php

namespace Statsig;

class StatsigOptions
{
    public $__ref = null;

    public function __construct(
        $specs_url = null,
        $log_event_url = null,
        $specs_adapter = null,
    ) {
        $ffi = StatsigFFI::get();
        $this->__ref = $ffi->statsig_options_create(
            $specs_url,
            $log_event_url,
            $specs_adapter->__file_ref
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
