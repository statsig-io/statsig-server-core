<?php

namespace Statsig;

abstract class SpecsAdapterBase
{
    public $__ref = null;

    public function __construct()
    {
        $ffi = StatsigFFI::get();
        $this->__ref = $ffi->function_based_specs_adapter_create(
            [$this, '_setup_internal'],
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

    public function _setup_internal(string $listener_ref)
    {
        $listener = new SpecsUpdateListener($listener_ref);
        $this->setup($listener);
    }

    public abstract function setup(SpecsUpdateListener $listener);

    public abstract function start();

    public abstract function shutdown();

    public abstract function scheduleBackgroundSync();
}
