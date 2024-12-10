<?php

namespace Statsig;

class StatsigLocalFileSpecsAdapter
{
    public ?string $__ref = null;

    public function __construct(
        string $sdk_key,
        string $output_directory,
        string $specs_url = null
    ) {
        $this->__ref = StatsigFFI::get()->statsig_local_file_specs_adapter_create($sdk_key, $output_directory, $specs_url);
    }

    public function __destruct()
    {
        if (!is_null($this->__ref)) {
            StatsigFFI::get()->statsig_local_file_specs_adapter_release($this->__ref);
        }

        $this->__ref = null;
    }

    public function sync_specs_from_network(): void
    {
        StatsigFFI::get()->statsig_local_file_specs_adapter_fetch_and_write_to_file($this->__ref);
    }
}
