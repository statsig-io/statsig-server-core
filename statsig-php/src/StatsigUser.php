<?php

namespace Statsig;

use FFI;

class StatsigUser
{
    public $__ref = null;

    public function __construct(string $user_id, string $email)
    {
        $ffi = StatsigFFI::get();
        $this->__ref = $ffi->statsig_user_create($user_id, $email);
    }

    public function __destruct()
    {
        if (is_null($this->__ref)) {
            return;
        }

        StatsigFFI::get()->statsig_user_ref_release($this->__ref);
        $this->__ref = null;
    }
}
