<?php

namespace Statsig;

class StatsigLocalFileSpecsAdapter
{
    public ?string $__ref = null;

    public function __construct(
        string $sdk_key,
        string $output_directory,
        string $specs_url = null,
        bool $fallback_to_statsig_api = false
    ) {
        $this->__ref = StatsigFFI::get()->statsig_local_file_specs_adapter_create($sdk_key, $output_directory, $specs_url, $fallback_to_statsig_api);
    }

    public function __destruct()
    {
        if (!is_null($this->__ref)) {
            StatsigFFI::get()->statsig_local_file_specs_adapter_release($this->__ref);
        }

        $this->__ref = null;
    }

    public function syncSpecsFromNetwork(): void
    {
        StatsigFFI::get()->statsig_local_file_specs_adapter_fetch_and_write_to_file($this->__ref);
    }
}
