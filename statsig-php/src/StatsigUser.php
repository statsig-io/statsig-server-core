<?php

namespace Statsig;

class StatsigUser
{
    public $__ref = null; // phpcs:ignore

    public function __construct(
        string $user_id,
        array $custom_ids = [],
        ?string $email = null,
        ?string $ip = null,
        ?string $user_agent = null,
        ?string $country = null,
        ?string $locale = null,
        ?string $app_version = null,
        ?array $custom = null,
        ?array $private_attributes = null
    ) {
        $ffi = StatsigFFI::get();
        $this->__ref = $ffi->statsig_user_create(
            $user_id,
            json_encode($custom_ids),
            $email,
            $ip,
            $user_agent,
            $country,
            $locale,
            $app_version,
            json_encode($custom),
            json_encode($private_attributes),
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
