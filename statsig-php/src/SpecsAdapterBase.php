<?php

namespace Statsig;

abstract class SpecsAdapterBase
{
    public $__ref = null; // phpcs:ignore

    public function __construct()
    {
        $ffi = StatsigFFI::get();
        $this->__ref = $ffi->function_based_specs_adapter_create(
            [$this, '_setupInternal'],
            [$this, 'start'],
            [$this, 'shutdown'],
            [$this, 'scheduleBackgroundSync']
        );
    }

    public function __destruct()
    {
        if (is_null($this->__ref)) {
            return;
        }

        $ffi = StatsigFFI::get();
        $ffi->function_based_specs_adapter_release($this->__ref);
        $this->__ref = null;
    }

    abstract public function setup(SpecsUpdateListener $listener);

    abstract public function start();

    abstract public function shutdown();

    abstract public function scheduleBackgroundSync();

    private function _setupInternal(string $listener_ref) // phpcs:ignore
    {
        $listener = new SpecsUpdateListener($listener_ref);
        $this->setup($listener);
    }
}
