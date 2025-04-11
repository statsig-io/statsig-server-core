<?php

namespace Statsig;

class SpecsUpdateListener
{
    public $__ref = null; // phpcs:ignore

    public function __construct(string $ref)
    {
        $this->__ref = $ref;
    }

    public function __destruct()
    {
        if (is_null($this->__ref)) {
            return;
        }

        $ffi = StatsigFFI::get();
        $ffi->specs_update_listener_release($this->__ref);
        $this->__ref = null;
    }

    public function didReceiveSpecsUpdate(string $update, string $source, int $received_at_ms): void
    {
        $ffi = StatsigFFI::get();
        $ffi->specs_update_listener_did_receive_specs_update(
            $this->__ref,
            $update,
            "Adapter($source)",
            $received_at_ms
        );
    }

    public function getCurrentSpecsInfo(): string
    {
        $ffi = StatsigFFI::get();
        return $ffi->specs_update_listener_get_current_specs_info($this->__ref);
    }
}
