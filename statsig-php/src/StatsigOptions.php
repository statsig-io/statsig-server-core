<?php

namespace Statsig;

class StatsigOptions
{
    public $__ref = null; // phpcs:ignore

    public function __construct(
        ?string $specs_url = null,
        ?string $log_event_url = null,
        ?object $specs_adapter = null,
        ?object $event_logging_adapter = null,
        ?string $environment = null,
        ?int $event_logging_flush_interval_ms = null,
        ?int $event_logging_max_queue_size = null,
        ?int $specs_sync_interval_ms = null,
        ?string $output_log_level = null,
        ?bool $disable_country_lookup = null,
        ?bool $wait_for_country_lookup_init = null,
        ?bool $wait_for_user_agent_init = null,
        ?bool $enable_id_lists = null,
        ?bool $disable_network = null,
        ?string $id_lists_url = null,
        ?int $id_lists_sync_interval_ms = null,
        ?bool $disable_all_logging = null,
        ?int $init_timeout_ms = null,
        ?bool $fallback_to_statsig_api = null,
        ?bool $use_third_party_ua_parser = null,
    ) {
        $ffi = StatsigFFI::get();
        $this->__ref = $ffi->statsig_options_create(
            $specs_url,
            $log_event_url,
            is_null($specs_adapter) ? 0 : $specs_adapter->__ref,
            is_null($event_logging_adapter) ? 0 : $event_logging_adapter->__ref,
            $environment,
            $event_logging_flush_interval_ms ?? -1,
            $event_logging_max_queue_size ?? -1,
            $specs_sync_interval_ms ?? -1,
            $output_log_level ?? null,
            toSafeOptBool($disable_country_lookup),
            toSafeOptBool($wait_for_country_lookup_init),
            toSafeOptBool($wait_for_user_agent_init),
            toSafeOptBool($enable_id_lists),
            toSafeOptBool($disable_network),
            $id_lists_url,
            $id_lists_sync_interval_ms ?? -1,
            toSafeOptBool($disable_all_logging),
            null, // global custom fields
            0, // ob client ref - not implemented in PHP
            0, // data store ref - not implemented in PHP
            $init_timeout_ms ?? -1,
            toSafeOptBool($fallback_to_statsig_api),
            toSafeOptBool($use_third_party_ua_parser)
        );
    }

    public function __destruct()
    {
        if (is_null($this->__ref)) {
            return;
        }

        StatsigFFI::get()->statsig_options_release($this->__ref);
        $this->__ref = null;
    }
}

function toSafeOptBool(?bool $value): int
{
    if (is_null($value)) {
        return -1;
    }

    return $value ? 1 : 0;
}
