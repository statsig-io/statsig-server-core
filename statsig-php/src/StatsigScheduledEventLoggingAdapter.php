<?php

namespace Statsig;

class StatsigScheduledEventLoggingAdapter
{
    public readonly string $file_path;
    public $__http_ref = null;
    public $__ref = null;

    public function __construct(string $file_path, string $sdk_key, StatsigOptions $options = null)
    {
        $this->file_path = $file_path;

        $options_ref = $options ? $options->__ref : (new StatsigOptions)->__ref;

        $this->__http_ref = StatsigFFI::get()->statsig_http_event_logging_adapter_create($sdk_key, $options_ref);
        $this->__ref = StatsigFFI::get()->statsig_local_file_event_logging_adapter_create($file_path);
    }

    public function __destruct()
    {
        if (!is_null($this->__http_ref)) {
            StatsigFFI::get()->statsig_http_event_logging_adapter_release($this->__http_ref);
        }

        if (!is_null($this->__ref)) {
            StatsigFFI::get()->statsig_local_file_event_logging_adapter_release($this->__ref);
        }

        $this->__http_ref = null;
        $this->__ref = null;
    }

    public function send_pending_events($callback)
    {
        if (!file_exists($this->file_path)) {
            $callback(false, "No events at path: " . $this->file_path);
            return;
        }

        $request_json = @file_get_contents($this->file_path);
        if ($request_json === false) {
            // Handle file reading error
            $err_msg = "Failed to read events from file: " . $this->file_path;
            error_log($err_msg);
            $callback(false, $err_msg);
            return;
        }

        StatsigFFI::get()->statsig_http_event_logging_adapter_send_events($this->__http_ref, $request_json, $callback);
    }
}
