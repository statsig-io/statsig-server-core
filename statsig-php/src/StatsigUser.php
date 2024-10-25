<?php

namespace Statsig;


class StatsigUser
{
    public $__ref = null;

    public function __construct(string $user_id, string $email = null)
    {
        $ffi = StatsigFFI::get();
        $this->__ref = $ffi->statsig_user_create(
            $user_id,
            null, // custom_ids
            $email,
            null, // ip
            null, // user_agent
            null, // country
            null, // locale
            null, // app_version
            null, // custom_json
            null, // private_attributes_json
        );
    }

    public function __destruct()
    {
        if (is_null($this->__ref)) {
            return;
        }

        StatsigFFI::get()->statsig_user_release($this->__ref);
        $this->__ref = null;
    }
}
