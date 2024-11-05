<?php

namespace Statsig;

class StatsigScheduledSpecsAdapter
{
    public $__ref = null;

    public function __construct(string $sdk_key, StatsigOptions $options = null)
    {
        $options_ref = $options ? $options->__ref : (new StatsigOptions)->__ref;

        $this->__ref = StatsigFFI::get()->statsig_http_specs_adapter_create($sdk_key, $options_ref);
    }

    public function __destruct()
    {
        if (is_null($this->__ref)) {
            return;
        }

        StatsigFFI::get()->statsig_http_specs_adapter_release($this->__ref);
        $this->__ref = null;
    }

    public function fetch_specs_from_network(): string
    {
        return StatsigFFI::get()->statsig_http_specs_adapter_fetch_specs_from_network($this->__ref, null);
    }
}
