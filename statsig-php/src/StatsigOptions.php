<?php

namespace Statsig;

class StatsigOptions
{
    public $__ref = null;

    public function __construct(
        $specs_url = null,
    )
    {
        $ffi = StatsigFFI::get();
        $this->__ref = $ffi->statsig_options_create(
            $specs_url
        );

        $a = 1;
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
