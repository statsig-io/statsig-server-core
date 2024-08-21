<?php

namespace Statsig\StatsigFFI;

use FFI;


class Statsig
{
    public $__ref = null;

    public function __construct(string $sdk_key)
    {
        $ffi = StatsigFFI::get();
        $this->__ref = $ffi->statsig_create($sdk_key, null);
    }

    public function __destruct()
    {
        if (is_null($this->__ref)) {
            return;
        }

        StatsigFFI::get()->statsig_ref_release($this->__ref);
        $this->__ref = null;
    }

    public function initialize($callback)
    {
        StatsigFFI::get()->statsig_initialize($this->__ref, $callback);
    }

    public function getClientInitializeResponse(StatsigUser $user): string
    {
        return StatsigFFI::get()->statsig_get_client_init_response($this->__ref, $user->__ref);
    }
}


