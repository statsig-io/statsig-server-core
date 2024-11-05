<?php

namespace Statsig;

class StatsigScheduledSpecsAdapter
{
    public readonly string $file_path;
    public $__http_ref = null;
    public $__file_ref = null;

    public function __construct(string $file_path, string $sdk_key, StatsigOptions $options = null)
    {
        $this->file_path = $file_path;

        $options_ref = $options ? $options->__ref : (new StatsigOptions)->__ref;

        $this->__http_ref = StatsigFFI::get()->statsig_http_specs_adapter_create($sdk_key, $options_ref);
        $this->__file_ref = StatsigFFI::get()->statsig_local_file_specs_adapter_create($file_path);
    }

    public function __destruct()
    {
        if (!is_null($this->__http_ref)) {
            StatsigFFI::get()->statsig_http_specs_adapter_release($this->__http_ref);
        }

        if (!is_null($this->__file_ref)) {
            StatsigFFI::get()->statsig_local_file_specs_adapter_release($this->__file_ref);
        }

        $this->__http_ref = null;
        $this->__file_ref = null;
    }

    public function sync_specs_from_network()
    {
        $specs_json = StatsigFFI::get()->statsig_http_specs_adapter_fetch_specs_from_network($this->__http_ref, null);
        if (is_null($specs_json)) {
            return;
        }

        try {
            $result = file_put_contents($this->file_path, $specs_json);
            if ($result === false) {
                error_log("Failed to write specs to file: " . $this->file_path);
            }
        } catch (\Exception $e) {
            error_log("Exception while writing specs to file: " . $e->getMessage());
        }
    }
}
