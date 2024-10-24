<?php

namespace Statsig;

class StatsigOptions
{
    public $__ref = null;

    public function __construct()
    {
        $ffi = StatsigFFI::get();
        $this->__ref = $ffi->statsig_options_create();

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
